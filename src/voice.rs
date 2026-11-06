//! One synth voice: oscillators + FM layer -> ladder filter -> amp.

use crate::dsp::*;
use crate::params::{fm_ratio, glide_seconds, Values, P};

/// Per-sample values shared by every voice, computed once by the engine.
pub struct Ctx {
    pub sr: f32,
    /// Global LFO, -1..1 (triangle).
    pub lfo: f32,
    /// Pitch multiplier from slow mode.
    pub rate: f32,
    /// Bend in semitones, plus mod-wheel vibrato depth already folded in.
    pub bend: f32,
    pub mod_wheel: f32,
}

pub struct Voice {
    pub note: u8,
    pub velocity: f32,
    pub gate: bool,
    /// Held only by the sustain pedal.
    pub sustained: bool,
    /// Monotonic start stamp, for choosing which voice to steal.
    pub started: u64,
    pitch: f32,
    target: f32,
    ph_saw: f32,
    ph_saw2: f32,
    ph_pulse: f32,
    ph_sub: f32,
    ph_car: f32,
    ph_mod: f32,
    fm_env: f32,
    lfo_fade: f32,
    drift: f32,
    drift_target: f32,
    drift_count: u32,
    amp: Adsr,
    fenv: Adsr,
    filter: Ladder,
    rng: Rng,
}

impl Voice {
    pub fn new(seed: u32) -> Self {
        let mut rng = Rng::new(seed.wrapping_mul(2_654_435_761).wrapping_add(1));
        // Start the oscillators out of phase with each other, like free-running analog.
        let mut r01 = || 0.5 + 0.5 * rng.bipolar();
        let (a, b, c, d) = (r01(), r01(), r01(), r01());
        Voice {
            note: 0,
            velocity: 0.0,
            gate: false,
            sustained: false,
            started: 0,
            pitch: 60.0,
            target: 60.0,
            ph_saw: a,
            ph_saw2: b,
            ph_pulse: c,
            ph_sub: d * 0.5,
            ph_car: 0.0,
            ph_mod: 0.0,
            fm_env: 0.0,
            lfo_fade: 0.0,
            drift: 0.0,
            drift_target: 0.0,
            drift_count: 0,
            amp: Adsr::default(),
            fenv: Adsr::default(),
            filter: Ladder::default(),
            rng,
        }
    }

    pub fn active(&self) -> bool {
        !self.amp.idle()
    }

    pub fn releasing(&self) -> bool {
        !self.gate && !self.sustained
    }

    /// `from` is the pitch to glide from (the last note played), if glide is on.
    pub fn note_on(&mut self, note: u8, velocity: f32, from: Option<f32>, stamp: u64) {
        let was_active = self.active();
        self.note = note;
        self.velocity = velocity;
        self.gate = true;
        self.sustained = false;
        self.started = stamp;
        self.target = note as f32;
        self.pitch = match from {
            Some(p) => p,
            None => note as f32,
        };
        if !was_active {
            self.filter.reset();
            self.lfo_fade = 0.0;
        }
        // FM operators restart in phase for a consistent attack transient, as on classic FM keyboards.
        self.ph_car = 0.0;
        self.ph_mod = 0.0;
        self.fm_env = 1.0;
        self.amp.gate_on();
        self.fenv.gate_on();
    }

    pub fn note_off(&mut self) {
        self.gate = false;
        if !self.sustained {
            self.amp.gate_off();
            self.fenv.gate_off();
        }
    }

    pub fn release_sustain(&mut self) {
        self.sustained = false;
        if !self.gate {
            self.amp.gate_off();
            self.fenv.gate_off();
        }
    }

    pub fn hold_for_sustain(&mut self) {
        if self.gate {
            self.sustained = true;
        }
    }

    pub fn amp_level(&self) -> f32 {
        self.amp.level
    }

    /// Renders one sample. `p` holds smoothed parameter values; `tscale` stretches
    /// envelope times in slow mode.
    #[inline]
    pub fn tick(&mut self, p: &Values, c: &Ctx, tscale: f32) -> f32 {
        let sr = c.sr;
        let v = |id: P| p[id as usize];

        // Glide toward the target pitch.
        let glide = glide_seconds(v(P::Glide));
        if glide > 0.0005 {
            self.pitch += (self.target - self.pitch) * (1.0 - (-4.0 / (glide * tscale * sr)).exp());
        } else {
            self.pitch = self.target;
        }

        // Slow random drift, a couple of cents, retargeted every ~1/3 s.
        if self.drift_count == 0 {
            self.drift_target = self.rng.bipolar() * 0.025;
            self.drift_count = (sr * 0.33) as u32;
        }
        self.drift_count -= 1;
        self.drift += (self.drift_target - self.drift) * 0.0002;

        // LFO fades in after the delay time, per voice.
        let delay = v(P::LfoDelay) * tscale;
        if delay < 0.01 {
            self.lfo_fade = 1.0;
        } else if self.lfo_fade < 1.0 {
            self.lfo_fade = (self.lfo_fade + 1.0 / (delay * sr)).min(1.0);
        }
        let lfo = c.lfo * self.lfo_fade;

        let vib = lfo * (v(P::Vibrato) * 0.5 + c.mod_wheel * 0.5);
        let semis = self.pitch + self.drift + vib + c.bend;
        let freq = midi_to_hz(semis) * c.rate;
        let dt = (freq / sr).min(0.45);

        // Subtractive oscillators.
        let mut mix = 0.0;
        let saw_l = v(P::Saw);
        let det = v(P::Detune);
        if saw_l > 0.0 {
            let s1 = saw(self.ph_saw, dt);
            let mut s = s1;
            if det > 0.05 {
                let dt2 = (dt * 2f32.powf(det / 1200.0)).min(0.45);
                let s2 = saw(self.ph_saw2, dt2);
                let w = (det / 4.0).min(1.0) * 0.5;
                s = s1 * (1.0 - w) + s2 * w;
                self.ph_saw2 = wrap01(self.ph_saw2 + dt2);
            }
            mix += saw_l * s;
        }
        let pulse_l = v(P::Pulse);
        if pulse_l > 0.0 {
            let pw = (v(P::PulseWidth) - v(P::Pwm) * 0.45 * (0.5 + 0.5 * lfo)).clamp(0.04, 0.5);
            mix += pulse_l * pulse(self.ph_pulse, dt, pw);
        }
        let sub_l = v(P::Sub);
        if sub_l > 0.0 {
            mix += sub_l * 0.8 * pulse(self.ph_sub, dt * 0.5, 0.5);
        }
        let noise_l = v(P::Noise);
        if noise_l > 0.0 {
            mix += noise_l * 0.6 * self.rng.bipolar();
        }
        self.ph_saw = wrap01(self.ph_saw + dt);
        self.ph_pulse = wrap01(self.ph_pulse + dt);
        self.ph_sub = wrap01(self.ph_sub + dt * 0.5);

        // 2-op FM layer: one sine modulating another, index decaying toward a floor.
        let fm_l = v(P::FmLevel);
        if fm_l > 0.0 {
            let ratio = fm_ratio(v(P::FmRatio));
            let decay = v(P::FmDecay) * tscale;
            self.fm_env += (0.12 - self.fm_env) * (1.0 - (-4.6 / (decay * sr)).exp());
            // Pull the index back on high notes so the sidebands stay under Nyquist.
            let guard = (3000.0 / (freq * ratio)).min(1.0);
            let index = v(P::FmIndex) * self.fm_env * (0.45 + 0.55 * self.velocity) * guard;
            let m = sin01(self.ph_mod);
            let car = sin01(wrap01(self.ph_car + index * m * 0.159_154_94));
            mix += fm_l * car;
            self.ph_mod = wrap01(self.ph_mod + dt * ratio);
            self.ph_car = wrap01(self.ph_car + dt);
        }

        // Filter.
        let fenv = self.fenv.next(
            v(P::FAttack) * tscale,
            v(P::FDecay) * tscale,
            v(P::FSustain),
            v(P::FRelease) * tscale,
            sr,
        );
        let octaves = v(P::FilterEnv) * fenv * 7.0
            + v(P::FilterLfo) * lfo * 2.5
            + v(P::KeyTrack) * (self.pitch - 60.0) / 12.0
            + v(P::FilterVel) * (self.velocity - 0.7) * 2.5;
        let cutoff = (v(P::Cutoff) * 2f32.powf(octaves) * c.rate.sqrt()).min(sr * 0.45);
        let k = v(P::Resonance) * 4.0;
        let y = self.filter.run(mix * 0.7, cutoff, k, sr);

        // Amp.
        let amp = self.amp.next(
            v(P::AAttack) * tscale,
            v(P::ADecay) * tscale,
            v(P::ASustain),
            v(P::ARelease) * tscale,
            sr,
        );
        let vs = v(P::AmpVel);
        let vel_gain = 1.0 - vs + vs * self.velocity * self.velocity * 1.3;
        y * amp * vel_gain
    }
}
