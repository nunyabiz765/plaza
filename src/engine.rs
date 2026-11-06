//! The whole instrument: voice allocation, global LFO, effects chain, master.
//!
//! `Engine` never allocates or locks after `new`, so it is safe to run inside
//! the audio callback. Everything else talks to it with `Event`s.

use crate::dsp::*;
use crate::fx::{Chorus, Reverb, Tape, TapeSettings};
use crate::params::{self, Values, P};
use crate::voice::{Ctx, Voice};

pub const VOICES: usize = 8;
const BLOCK: usize = 32;
const VOICE_GAIN: f32 = 0.48;

#[derive(Clone, Copy, Debug)]
pub enum Event {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
    Param(P, f32),
    Load(Values),
    Slow(bool),
    /// Pitch bend in semitones.
    Bend(f32),
    ModWheel(f32),
    Sustain(bool),
    AllNotesOff,
}

pub struct Engine {
    sr: f32,
    voices: Vec<Voice>,
    target: Values,
    smooth: Values,
    slow: bool,
    rate: f32,
    lfo_phase: f32,
    chorus: Chorus,
    reverb: Reverb,
    tape: Tape,
    bend: f32,
    mod_wheel: f32,
    sustain: bool,
    stamp: u64,
    last_pitch: Option<f32>,
    dc_l: DcBlock,
    dc_r: DcBlock,
    gain: f32,
}

impl Engine {
    pub fn new(sr: f32) -> Self {
        let v = params::defaults();
        Engine {
            sr,
            voices: (0..VOICES as u32).map(Voice::new).collect(),
            target: v,
            smooth: v,
            slow: false,
            rate: 1.0,
            lfo_phase: 0.0,
            chorus: Chorus::new(sr),
            reverb: Reverb::new(sr),
            tape: Tape::new(sr),
            bend: 0.0,
            mod_wheel: 0.0,
            sustain: false,
            stamp: 0,
            last_pitch: None,
            dc_l: DcBlock::new(sr),
            dc_r: DcBlock::new(sr),
            gain: db_to_gain(v[P::Volume as usize]),
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sr
    }

    /// Applies a preset immediately, without gliding there (for offline renders).
    pub fn load_now(&mut self, v: &Values) {
        self.target = *v;
        self.smooth = *v;
        self.gain = db_to_gain(v[P::Volume as usize]);
    }

    pub fn set_slow_now(&mut self, on: bool) {
        self.slow = on;
        self.rate = if on { self.target[P::SlowRate as usize] } else { 1.0 };
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active()).count()
    }

    pub fn handle(&mut self, e: Event) {
        match e {
            Event::NoteOn { note, velocity } if velocity <= 0.0 => self.note_off(note),
            Event::NoteOn { note, velocity } => self.note_on(note, velocity.min(1.0)),
            Event::NoteOff { note } => self.note_off(note),
            Event::Param(p, v) => {
                let d = p.def();
                self.target[p as usize] = v.clamp(d.min, d.max);
                if p.is_stepped() {
                    self.smooth[p as usize] = self.target[p as usize];
                }
            }
            Event::Load(v) => {
                self.target = v;
                for &p in params::ALL {
                    if p.is_stepped() {
                        self.smooth[p as usize] = v[p as usize];
                    }
                }
            }
            Event::Slow(on) => self.slow = on,
            Event::Bend(s) => self.bend = s,
            Event::ModWheel(m) => self.mod_wheel = m.clamp(0.0, 1.0),
            Event::Sustain(on) => {
                self.sustain = on;
                if !on {
                    for v in &mut self.voices {
                        v.release_sustain();
                    }
                }
            }
            Event::AllNotesOff => {
                self.sustain = false;
                for v in &mut self.voices {
                    v.gate = false;
                    v.release_sustain();
                }
            }
        }
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        self.stamp += 1;
        let glide = self.target[P::Glide as usize] > 0.001;
        let from = if glide { self.last_pitch } else { None };
        // Same note still sounding: retrigger it rather than doubling up.
        let idx = self
            .voices
            .iter()
            .position(|v| v.active() && v.note == note)
            .or_else(|| self.voices.iter().position(|v| !v.active()))
            .or_else(|| {
                // Quietest released voice, then the oldest held one.
                self.voices
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v.releasing())
                    .min_by(|a, b| a.1.amp_level().total_cmp(&b.1.amp_level()))
                    .map(|(i, _)| i)
            })
            .unwrap_or_else(|| {
                self.voices.iter().enumerate().min_by_key(|(_, v)| v.started).map(|(i, _)| i).unwrap_or(0)
            });
        self.voices[idx].note_on(note, velocity, from, self.stamp);
        self.last_pitch = Some(note as f32);
    }

    fn note_off(&mut self, note: u8) {
        for v in &mut self.voices {
            if v.gate && v.note == note {
                if self.sustain {
                    v.hold_for_sustain();
                }
                v.note_off();
            }
        }
    }

    /// Renders interleaved stereo into `out` (length = 2 * frames).
    pub fn render(&mut self, out: &mut [f32]) {
        for chunk in out.chunks_mut(BLOCK * 2) {
            self.render_block(chunk);
        }
    }

    fn render_block(&mut self, out: &mut [f32]) {
        let sr = self.sr;
        let frames = out.len() / 2;
        // Parameter glide: ~15 ms, so knob moves and preset changes never click.
        let k = 1.0 - (-(frames as f32) / (0.015 * sr)).exp();
        for &p in params::ALL {
            let i = p as usize;
            if !p.is_stepped() {
                self.smooth[i] += (self.target[i] - self.smooth[i]) * k;
            }
        }
        let gain_target = db_to_gain(self.smooth[P::Volume as usize]);
        let rate_target = if self.slow { self.target[P::SlowRate as usize] } else { 1.0 };
        // Slow mode eases in like a tape motor spinning down (~0.35 s).
        let kr = 1.0 - (-(frames as f32) / (0.35 * sr)).exp();
        self.rate += (rate_target - self.rate) * kr;
        let tscale = 1.0 / self.rate;

        let p = self.smooth;
        let v = |id: P| p[id as usize];
        let chorus_mode = v(P::Chorus) as usize;
        let rev_mix = v(P::RevMix);
        let rev_decay = v(P::RevDecay) * tscale;
        let rev_damp = v(P::RevDamp);
        let rev_pre = v(P::RevPre);
        let tape = TapeSettings {
            wow: v(P::Wow),
            flutter: v(P::Flutter),
            saturation: v(P::Saturation),
            age: v(P::Age),
            hiss: v(P::Hiss),
            crush: v(P::Crush),
            rate: self.rate,
        };
        let lfo_rate = v(P::LfoRate) * self.rate;
        let dry = 1.0 - 0.45 * rev_mix;
        let wet = 1.6 * rev_mix;

        for f in 0..frames {
            self.lfo_phase = wrap01(self.lfo_phase + lfo_rate / sr);
            let lfo = 4.0 * (self.lfo_phase - 0.5).abs() - 1.0;
            let ctx = Ctx { sr, lfo, rate: self.rate, bend: self.bend, mod_wheel: self.mod_wheel };
            let mut mono = 0.0;
            for voice in &mut self.voices {
                if voice.active() {
                    mono += voice.tick(&p, &ctx, tscale);
                }
            }
            mono *= VOICE_GAIN;

            let (cl, cr) = self.chorus.run(mono, chorus_mode);
            let (wl, wr) = self.reverb.run(cl, cr, rev_decay, rev_damp, rev_pre);
            let (l, r) = (cl * dry + wl * wet, cr * dry + wr * wet);
            let (l, r) = self.tape.run(l, r, &tape);

            self.gain += (gain_target - self.gain) * 0.001;
            let l = limit(self.dc_l.run(l * self.gain));
            let r = limit(self.dc_r.run(r * self.gain));
            out[2 * f] = l;
            out[2 * f + 1] = r;
        }
    }
}

/// Transparent below -2 dBFS, then a soft knee that never exceeds 1.0.
#[inline]
pub fn limit(x: f32) -> f32 {
    const T: f32 = 0.8;
    let a = x.abs();
    if a <= T {
        x
    } else {
        let y = T + (1.0 - T) * soft_clip((a - T) / (1.0 - T));
        y.copysign(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(e: &mut Engine, secs: f32) -> Vec<f32> {
        let mut buf = vec![0.0; (e.sample_rate() * secs) as usize * 2];
        e.render(&mut buf);
        buf
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn silence_when_idle() {
        let mut e = Engine::new(48_000.0);
        let out = run(&mut e, 0.5);
        assert!(out.iter().all(|s| s.abs() < 1e-6));
    }

    #[test]
    fn plays_the_right_pitch() {
        let sr = 48_000.0;
        let mut e = Engine::new(sr);
        let mut v = params::defaults();
        // Pure-ish tone: FM layer only with no modulation, filter wide open, dry.
        v[P::Saw as usize] = 0.0;
        v[P::FmLevel as usize] = 1.0;
        v[P::FmIndex as usize] = 0.0;
        v[P::Cutoff as usize] = 18_000.0;
        v[P::RevMix as usize] = 0.0;
        e.load_now(&v);
        e.handle(Event::NoteOn { note: 69, velocity: 0.8 });
        let out = run(&mut e, 1.0);
        let left: Vec<f32> = out.iter().step_by(2).copied().collect();
        let zc = left[4800..].windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32;
        let hz = zc / ((left.len() - 4800) as f32 / sr);
        assert!((hz - 440.0).abs() < 3.0, "{hz}");
    }

    #[test]
    fn slow_mode_drops_pitch_by_the_rate() {
        let sr = 48_000.0;
        let mut e = Engine::new(sr);
        let mut v = params::defaults();
        v[P::Saw as usize] = 0.0;
        v[P::FmLevel as usize] = 1.0;
        v[P::FmIndex as usize] = 0.0;
        v[P::RevMix as usize] = 0.0;
        v[P::SlowRate as usize] = 0.8;
        e.load_now(&v);
        e.set_slow_now(true);
        e.handle(Event::NoteOn { note: 69, velocity: 0.8 });
        let out = run(&mut e, 1.0);
        let left: Vec<f32> = out.iter().step_by(2).copied().collect();
        let zc = left[4800..].windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32;
        let hz = zc / ((left.len() - 4800) as f32 / sr);
        assert!((hz - 352.0).abs() < 3.0, "{hz}");
    }

    #[test]
    fn polyphony_and_stealing() {
        let mut e = Engine::new(48_000.0);
        for n in 0..12 {
            e.handle(Event::NoteOn { note: 48 + n, velocity: 0.7 });
            run(&mut e, 0.01);
        }
        assert_eq!(e.active_voices(), VOICES);
        // The newest notes survive the steal.
        let notes: Vec<u8> = e.voices.iter().map(|v| v.note).collect();
        for n in 52..60 {
            assert!(notes.contains(&n), "{notes:?}");
        }
        e.handle(Event::AllNotesOff);
        run(&mut e, 3.0);
        assert_eq!(e.active_voices(), 0);
    }

    #[test]
    fn sustain_pedal_holds_notes() {
        let mut e = Engine::new(48_000.0);
        e.handle(Event::Sustain(true));
        e.handle(Event::NoteOn { note: 60, velocity: 0.7 });
        run(&mut e, 0.05);
        e.handle(Event::NoteOff { note: 60 });
        run(&mut e, 2.0);
        assert_eq!(e.active_voices(), 1);
        e.handle(Event::Sustain(false));
        run(&mut e, 3.0);
        assert_eq!(e.active_voices(), 0);
    }

    #[test]
    fn retrigger_reuses_the_voice() {
        let mut e = Engine::new(48_000.0);
        e.handle(Event::NoteOn { note: 60, velocity: 0.7 });
        run(&mut e, 0.05);
        e.handle(Event::NoteOn { note: 60, velocity: 0.7 });
        assert_eq!(e.active_voices(), 1);
    }

    #[test]
    fn limiter_never_exceeds_full_scale() {
        for i in -400..=400 {
            let x = i as f32 * 0.05;
            assert!(limit(x).abs() <= 1.0);
        }
        assert_eq!(limit(0.5), 0.5);
    }

    #[test]
    fn full_chord_at_full_volume_stays_under_zero_dbfs() {
        let mut e = Engine::new(48_000.0);
        let mut v = params::defaults();
        v[P::Volume as usize] = 6.0;
        v[P::Saw as usize] = 1.0;
        v[P::Pulse as usize] = 1.0;
        v[P::Sub as usize] = 1.0;
        v[P::Resonance as usize] = 1.0;
        v[P::Saturation as usize] = 1.0;
        v[P::RevMix as usize] = 1.0;
        e.load_now(&v);
        for n in [36, 48, 55, 60, 64, 67, 71, 72] {
            e.handle(Event::NoteOn { note: n, velocity: 1.0 });
        }
        let out = run(&mut e, 2.0);
        assert!(out.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
        assert!(rms(&out) > 0.05);
    }

    #[test]
    fn param_changes_do_not_click() {
        // A cutoff slam mid-note should not produce a sample-to-sample jump
        // bigger than the signal itself makes.
        let mut e = Engine::new(48_000.0);
        e.handle(Event::NoteOn { note: 57, velocity: 0.8 });
        let a = run(&mut e, 0.5);
        let max_step = |x: &[f32]| x.chunks(2).map(|f| f[0]).collect::<Vec<_>>().windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max);
        let before = max_step(&a[24_000..]);
        e.handle(Event::Param(P::Cutoff, 60.0));
        e.handle(Event::Param(P::Volume, 6.0));
        let b = run(&mut e, 0.1);
        assert!(max_step(&b) <= before * 1.6 + 1e-3, "{} vs {before}", max_step(&b));
    }
}
