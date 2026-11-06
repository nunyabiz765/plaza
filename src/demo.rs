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

// Sound pack 1 demos, 8 bars in D minor: Dm9 - Bbmaj7 - Fadd9 - Csus2 - Gm9 - Bbmaj9 - Fmaj7 - Asus4.
// Mid-register voicings; the bass parts supply the roots.
const PACK_CHORDS: [[u8; 4]; 8] = [
    [53, 57, 62, 64],
    [53, 57, 62, 65],
    [55, 57, 60, 65],
    [55, 60, 62, 67],
    [53, 57, 58, 62],
    [53, 57, 60, 62],
    [52, 57, 60, 64],
    [52, 57, 62, 64],
];

/// A bass written to be played high, as the tune: eighth notes, one bar per row.
const HI_BASS: [[u8; 8]; 8] = [
    [50, 50, 57, 50, 62, 60, 57, 55],
    [46, 46, 53, 46, 58, 57, 53, 50],
    [53, 53, 60, 53, 65, 64, 60, 57],
    [48, 48, 55, 48, 60, 62, 64, 67],
    [55, 55, 62, 55, 65, 62, 58, 57],
    [58, 57, 53, 50, 58, 60, 62, 65],
    [65, 64, 60, 57, 64, 60, 57, 53],
    [57, 57, 64, 57, 62, 64, 57, 52],
];

pub const PHRASES: &[&str] = &[
    "chords", "keys", "melody", "stabs", "bass", "scale", "pads8", "seq16", "hibass", "topline", "stabs8", "strings8",
];

/// A phrase by name. `NAME*N` plays it N times back to back (whole bars).
pub fn phrase(name: &str) -> Option<Vec<Note>> {
    if let Some((base, times)) = name.split_once('*') {
        let times: usize = times.parse().ok().filter(|t| (1..=16).contains(t))?;
        let one = phrase(base)?;
        let end = one.iter().fold(0f32, |m, nt| m.max(nt.start + nt.len));
        let bars = (end / 4.0).ceil().max(1.0) * 4.0;
        return Some((0..times).flat_map(|i| one.iter().map(move |nt| Note { start: nt.start + i as f32 * bars, ..*nt })).collect());
    }
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
        "pads8" => {
            for (bar, chord) in PACK_CHORDS.iter().enumerate() {
                for (i, &k) in chord.iter().enumerate() {
                    v.push(n(bar as f32 * 4.0 + i as f32 * 0.01, 3.95, k, 0.72));
                }
            }
        }
        "strings8" => {
            // Legato: each chord overlaps the next a touch, top voice doubled an octave up.
            for (bar, chord) in PACK_CHORDS.iter().enumerate() {
                let len = if bar == 7 { 3.9 } else { 4.1 };
                for &k in chord {
                    v.push(n(bar as f32 * 4.0, len, k, 0.65));
                }
                v.push(n(bar as f32 * 4.0, len, chord[3] + 12, 0.6));
            }
        }
        "seq16" => {
            // Sixteenths over the chord an octave up; accents on the beat, lighter offbeats.
            const PAT: [usize; 16] = [0, 2, 1, 3, 2, 4, 3, 1, 0, 2, 1, 3, 4, 3, 2, 1];
            for (bar, chord) in PACK_CHORDS.iter().enumerate() {
                let tones = [chord[0] + 12, chord[1] + 12, chord[2] + 12, chord[3] + 12, chord[0] + 24];
                for (i, &ix) in PAT.iter().enumerate() {
                    let vel = match i % 4 {
                        0 => 0.95,
                        2 => 0.75,
                        _ => 0.6,
                    };
                    v.push(n(bar as f32 * 4.0 + i as f32 * 0.25, 0.2, tones[ix], vel));
                }
            }
        }
        "hibass" => {
            for (bar, row) in HI_BASS.iter().enumerate() {
                for (i, &k) in row.iter().enumerate() {
                    let vel = if i % 2 == 0 { 0.9 } else { 0.7 };
                    v.push(n(bar as f32 * 4.0 + i as f32 * 0.5, 0.42, k, vel));
                }
            }
        }
        "topline" => {
            let line = [
                (0.0, 2.0, 81), (2.0, 1.0, 77), (3.0, 1.0, 76),
                (4.0, 3.0, 74), (7.0, 1.0, 77),
                (8.0, 1.5, 76), (9.5, 0.5, 77), (10.0, 2.0, 72),
                (12.0, 2.0, 74), (14.0, 2.0, 76),
                (16.0, 2.0, 77), (18.0, 1.0, 74), (19.0, 1.0, 72),
                (20.0, 3.0, 74), (23.0, 1.0, 72),
                (24.0, 1.5, 69), (25.5, 0.5, 72), (26.0, 2.0, 76),
                (28.0, 3.9, 76),
            ];
            for (s, l, k) in line {
                v.push(n(s, l, k, 0.8));
            }
        }
        "stabs8" => {
            for (bar, chord) in PACK_CHORDS.iter().enumerate() {
                let b = bar as f32 * 4.0;
                let mut hits = vec![(0.0, 0.95), (1.5, 0.8), (3.0, 0.85)];
                if bar % 2 == 1 {
                    hits.push((3.5, 0.7));
                }
                for (at, vel) in hits {
                    for &k in chord {
                        v.push(n(b + at, 0.3, k + 12, vel));
                    }
                }
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
    fn phrases_exist_and_repeat() {
        for &name in PHRASES {
            assert!(!phrase(name).unwrap().is_empty(), "{name}");
        }
        let one = phrase("seq16").unwrap();
        let two = phrase("seq16*2").unwrap();
        assert_eq!(two.len(), one.len() * 2);
        assert_eq!(two[one.len()].start, 32.0);
        assert!(phrase("seq16*0").is_none() && phrase("nope*2").is_none());
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
