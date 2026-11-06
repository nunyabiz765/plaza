//! Offline rendering: short built-in phrases played through the engine, for
//! demos and for checking the sound without touching the speakers.

use crate::engine::{limit, Engine, Event};
use crate::params::{Values, P};

#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub start: f32,
    pub len: f32,
    pub note: u8,
    pub vel: f32,
}

const fn n(start: f32, len: f32, note: u8, vel: f32) -> Note {
    Note { start, len, note, vel }
}

// Fmaj9 - Em7 - Dm9 - Cmaj7: the evergreen mall progression.
const CHORDS: [[u8; 5]; 4] = [[53, 57, 60, 64, 67], [52, 55, 59, 62, 0], [50, 53, 57, 60, 64], [48, 55, 59, 64, 0]];
const ROOTS: [u8; 4] = [41, 40, 38, 36];

pub const PHRASES: &[&str] = &["chords", "keys", "melody", "stabs", "bass", "scale"];

pub fn phrase(name: &str) -> Option<Vec<Note>> {
    let mut v = Vec::new();
    match name {
        "chords" => {
            for (bar, chord) in CHORDS.iter().enumerate() {
                for (i, &k) in chord.iter().enumerate().filter(|(_, &k)| k > 0) {
                    v.push(n(bar as f32 * 4.0 + i as f32 * 0.02, 3.9, k, 0.7));
                }
            }
        }
        "keys" => {
            // Comping: a held chord, a pushed hit on the "and" of 2, a ghost on 4.
            let hits = [(0.0, 1.4, 0.78), (1.5, 0.9, 0.62), (3.0, 0.45, 0.5)];
            for (bar, chord) in CHORDS.iter().enumerate() {
                for &(at, len, vel) in &hits {
                    for (i, &k) in chord.iter().enumerate().filter(|(_, &k)| k > 0) {
                        v.push(n(bar as f32 * 4.0 + at + i as f32 * 0.012, len, k, vel - i as f32 * 0.03));
                    }
                }
            }
        }
        "melody" => {
            let line = [
                (0.0, 1.5, 76), (1.5, 0.5, 74), (2.0, 2.0, 72),
                (4.0, 1.0, 71), (5.0, 1.0, 74), (6.0, 2.0, 79),
                (8.0, 1.5, 77), (9.5, 0.5, 76), (10.0, 1.0, 74), (11.0, 1.0, 72),
                (12.0, 1.0, 71), (13.0, 2.8, 76),
            ];
            for (s, l, k) in line {
                v.push(n(s, l, k, 0.75));
            }
        }
        "stabs" => {
            for (bar, chord) in CHORDS.iter().enumerate() {
                for &at in &[0.0, 1.5, 2.5] {
                    for &k in chord.iter().filter(|&&k| k > 0) {
                        v.push(n(bar as f32 * 4.0 + at, 0.35, k + 12, 0.9));
                    }
                }
            }
        }
        "bass" => {
            for (bar, &r) in ROOTS.iter().enumerate() {
                let b = bar as f32 * 4.0;
                v.push(n(b, 1.4, r, 0.85));
                v.push(n(b + 1.5, 0.4, r + 12, 0.6));
                v.push(n(b + 2.0, 1.4, r, 0.75));
                v.push(n(b + 3.5, 0.4, r + 7, 0.6));
            }
        }
        "scale" => {
            for (i, k) in [60u8, 62, 64, 65, 67, 69, 71, 72].iter().enumerate() {
                v.push(n(i as f32 * 0.5, 0.45, *k, 0.7));
            }
        }
        _ => return None,
    }
    Some(v)
}

pub struct Layer<'a> {
    pub values: &'a Values,
    pub notes: &'a [Note],
}

/// Renders layers (each its own engine and effects) and sums them. If `slow`
/// is on, timing stretches by the preset's slow rate too, like a slowed record.
/// Returns interleaved stereo.
pub fn render(layers: &[Layer], bpm: f32, slow: bool, sr: f32, tail: f32) -> Vec<f32> {
    const BLOCK: usize = 64;
    let mut engines: Vec<(Engine, Vec<(usize, Event)>)> = layers
        .iter()
        .map(|l| {
            let mut e = Engine::new(sr);
            e.load_now(l.values);
            e.set_slow_now(slow);
            let stretch = if slow { 1.0 / l.values[P::SlowRate as usize] } else { 1.0 };
            let spb = 60.0 / bpm * stretch * sr;
            let mut ev = Vec::new();
            for nt in l.notes {
                ev.push(((nt.start * spb) as usize, Event::NoteOn { note: nt.note, velocity: nt.vel }));
                ev.push((((nt.start + nt.len) * spb) as usize, Event::NoteOff { note: nt.note }));
            }
            ev.sort_by_key(|(t, e)| (*t, matches!(e, Event::NoteOn { .. })));
            ev.reverse();
            (e, ev)
        })
        .collect();
    let end = engines.iter().filter_map(|(_, ev)| ev.first().map(|x| x.0)).max().unwrap_or(0);
    let total = end + (tail * sr) as usize;
    let mut out = vec![0.0f32; total * 2];
    let mut buf = vec![0.0f32; BLOCK * 2];
    let mut t = 0;
    while t < total {
        let frames = BLOCK.min(total - t);
        for (e, ev) in engines.iter_mut() {
            while ev.last().is_some_and(|(at, _)| *at <= t) {
                e.handle(ev.pop().unwrap().1);
            }
            e.render(&mut buf[..frames * 2]);
            for (o, s) in out[t * 2..(t + frames) * 2].iter_mut().zip(&buf) {
                *o += s;
            }
        }
        t += frames;
    }
    if layers.len() > 1 {
        out.iter_mut().for_each(|s| *s = limit(*s));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets;

    fn stats(x: &[f32]) -> (f32, f32) {
        let peak = x.iter().fold(0f32, |m, s| m.max(s.abs()));
        let rms = (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt();
        (peak, rms)
    }

    /// Every preset, on the phrase it's meant for, renders clean, audible and
    /// below full scale, with and without slow mode.
    #[test]
    fn every_preset_renders_sensibly() {
        for p in presets::all() {
            let ph = if p.name.contains("Bass") { "bass" } else { "chords" };
            let notes = phrase(ph).unwrap();
            for slow in [false, true] {
                let out = render(&[Layer { values: &p.values, notes: &notes }], 90.0, slow, 48_000.0, 2.0);
                assert!(out.iter().all(|s| s.is_finite()), "{}", p.name);
                let (peak, rms) = stats(&out);
                let rms_db = 20.0 * rms.log10();
                assert!(peak <= 1.0, "{} peak {peak}", p.name);
                assert!((-32.0..-8.0).contains(&rms_db), "{} slow={slow} rms {rms_db:.1} dBFS peak {peak}", p.name);
            }
        }
    }

    #[test]
    fn renders_are_deterministic() {
        let p = &presets::all()[0];
        let notes = phrase("scale").unwrap();
        let a = render(&[Layer { values: &p.values, notes: &notes }], 120.0, false, 48_000.0, 0.5);
        let b = render(&[Layer { values: &p.values, notes: &notes }], 120.0, false, 48_000.0, 0.5);
        assert_eq!(a, b);
    }

    /// The release tail actually dies out: no stuck notes, no runaway reverb.
    #[test]
    fn tails_fade_to_silence() {
        for p in presets::all() {
            let notes = phrase("scale").unwrap();
            let out = render(&[Layer { values: &p.values, notes: &notes }], 120.0, false, 48_000.0, 30.0);
            let last = &out[out.len() - 48_000..];
            let (peak, _) = stats(last);
            // Hiss is deliberate; allow it.
            assert!(peak < 0.02, "{} still sounding: {peak}", p.name);
        }
    }
}
