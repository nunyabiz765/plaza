//! Factory presets. Everything is synthesized; there are no samples anywhere.

use crate::params::{self, Values, P};
use P::*;

pub struct Preset {
    pub name: &'static str,
    pub blurb: &'static str,
    pub values: Values,
}

fn build(overrides: &[(P, f32)]) -> Values {
    let mut v = params::defaults();
    for &(p, x) in overrides {
        v[p as usize] = x;
    }
    v
}

/// FM_RATIOS index helpers, so presets read as ratios.
const R1: f32 = 1.0;
const R2: f32 = 3.0;
const R3_5: f32 = 5.0;
const R7: f32 = 8.0;
const R14: f32 = 10.0;

pub fn all() -> Vec<Preset> {
    vec![
        Preset {
            name: "Chorus Pad",
            blurb: "Saw + PWM pulse and a sub through chorus II. The 1984 string machine everyone sampled.",
            values: build(&[
                (Saw, 0.75), (Pulse, 0.55), (PulseWidth, 0.42), (Pwm, 0.55), (Sub, 0.35),
                (Cutoff, 2400.0), (Resonance, 0.12), (FilterEnv, 0.18), (KeyTrack, 0.4), (FilterVel, 0.1),
                (FAttack, 0.9), (FDecay, 2.5), (FSustain, 0.5), (FRelease, 2.0),
                (AAttack, 0.45), (ADecay, 1.5), (ASustain, 0.9), (ARelease, 1.6), (AmpVel, 0.25),
                (LfoRate, 0.55), (Chorus, 2.0),
                (RevDecay, 4.5), (RevDamp, 0.45), (RevPre, 25.0), (RevMix, 0.3),
                (Wow, 0.15), (Flutter, 0.15), (Age, 0.15), (Volume, -7.0),
            ]),
        },
        Preset {
            name: "FM Tine Piano",
            blurb: "Two-operator FM electric piano: 1:1 bark that softens as the index decays, chorus I.",
            values: build(&[
                (Saw, 0.0), (FmLevel, 0.9), (FmRatio, R1), (FmIndex, 3.2), (FmDecay, 0.55),
                (Cutoff, 9000.0), (Resonance, 0.0), (FilterEnv, 0.1), (KeyTrack, 0.3), (FilterVel, 0.35),
                (FAttack, 0.002), (FDecay, 1.2), (FSustain, 0.0), (FRelease, 0.4),
                (AAttack, 0.002), (ADecay, 3.5), (ASustain, 0.0), (ARelease, 0.45), (AmpVel, 0.6),
                (LfoRate, 5.0), (Chorus, 1.0),
                (RevDecay, 2.6), (RevDamp, 0.5), (RevPre, 15.0), (RevMix, 0.22),
                (Wow, 0.08), (Flutter, 0.1), (Volume, -3.0),
            ]),
        },
        Preset {
            name: "Crystal Bell",
            blurb: "FM at 3.5:1 for an inharmonic, glassy strike with a long ring-out.",
            values: build(&[
                (Saw, 0.0), (FmLevel, 0.85), (FmRatio, R3_5), (FmIndex, 3.5), (FmDecay, 1.6),
                (Cutoff, 12000.0), (FilterEnv, 0.0), (KeyTrack, 0.0),
                (AAttack, 0.001), (ADecay, 5.0), (ASustain, 0.0), (ARelease, 2.5), (AmpVel, 0.5),
                (Chorus, 1.0), (RevDecay, 6.0), (RevDamp, 0.3), (RevPre, 40.0), (RevMix, 0.38),
                (Volume, -1.0),
            ]),
        },
        Preset {
            name: "Brass Stab",
            blurb: "Detuned saws and a snappy filter envelope. Hit chords short.",
            values: build(&[
                (Saw, 0.95), (Detune, 9.0), (Pulse, 0.25), (PulseWidth, 0.3), (Sub, 0.2),
                (Cutoff, 520.0), (Resonance, 0.22), (FilterEnv, 0.62), (KeyTrack, 0.5), (FilterVel, 0.45),
                (FAttack, 0.035), (FDecay, 0.38), (FSustain, 0.35), (FRelease, 0.25),
                (AAttack, 0.012), (ADecay, 0.5), (ASustain, 0.75), (ARelease, 0.22), (AmpVel, 0.5),
                (LfoRate, 5.5), (LfoDelay, 0.5), (Vibrato, 0.08), (Chorus, 1.0),
                (RevDecay, 1.8), (RevDamp, 0.5), (RevMix, 0.18), (Saturation, 0.25), (Volume, -3.0),
            ]),
        },
        Preset {
            name: "Glass Lead",
            blurb: "Narrow pulse over a saw, poly glide, delayed vibrato. Sings over a pad.",
            values: build(&[
                (Saw, 0.45), (Pulse, 0.8), (PulseWidth, 0.18), (Pwm, 0.15), (FmLevel, 0.25), (FmRatio, R2),
                (FmIndex, 1.0), (FmDecay, 0.3),
                (Cutoff, 3200.0), (Resonance, 0.35), (FilterEnv, 0.3), (KeyTrack, 0.7),
                (FAttack, 0.01), (FDecay, 0.7), (FSustain, 0.45), (FRelease, 0.4),
                (AAttack, 0.01), (ADecay, 0.6), (ASustain, 0.85), (ARelease, 0.35), (AmpVel, 0.4),
                (Glide, 0.25), (LfoRate, 5.2), (LfoDelay, 0.45), (Vibrato, 0.3), (Chorus, 1.0),
                (RevDecay, 3.2), (RevPre, 60.0), (RevMix, 0.28), (Volume, -2.0),
            ]),
        },
        Preset {
            name: "Mall Atrium",
            blurb: "Slow, dark, enormous. Fountain-court reverb, a little hiss, tape that's been in the sun.",
            values: build(&[
                (Saw, 0.6), (Pulse, 0.45), (PulseWidth, 0.45), (Pwm, 0.4), (Sub, 0.25), (Noise, 0.04), (Detune, 6.0),
                (Cutoff, 1300.0), (Resonance, 0.08), (FilterEnv, 0.15), (FilterLfo, 0.08), (KeyTrack, 0.3),
                (FAttack, 2.5), (FDecay, 4.0), (FSustain, 0.6), (FRelease, 4.0),
                (AAttack, 1.6), (ADecay, 3.0), (ASustain, 0.95), (ARelease, 4.5), (AmpVel, 0.15),
                (LfoRate, 0.18), (Chorus, 1.0),
                (RevDecay, 11.0), (RevDamp, 0.55), (RevPre, 80.0), (RevMix, 0.55),
                (Wow, 0.35), (Flutter, 0.25), (Saturation, 0.2), (Age, 0.45), (Hiss, 0.25), (Volume, -7.0),
            ]),
        },
        Preset {
            name: "Floral Strings",
            blurb: "Bright ensemble with the I+II shimmer. Lush on sustained chords.",
            values: build(&[
                (Saw, 0.9), (Detune, 12.0), (Pulse, 0.3), (Pwm, 0.3), (PulseWidth, 0.4),
                (Cutoff, 5200.0), (Resonance, 0.05), (FilterEnv, 0.08), (KeyTrack, 0.5),
                (FAttack, 0.3), (FDecay, 2.0), (FSustain, 0.7), (FRelease, 1.2),
                (AAttack, 0.28), (ADecay, 1.0), (ASustain, 0.92), (ARelease, 1.2), (AmpVel, 0.3),
                (LfoRate, 4.8), (LfoDelay, 0.8), (Vibrato, 0.05), (Chorus, 3.0),
                (RevDecay, 3.8), (RevMix, 0.3), (Wow, 0.1), (Age, 0.1), (Volume, -6.0),
            ]),
        },
        Preset {
            name: "Cassette Keys",
            blurb: "Pulse-and-FM keys sampled to a cheap sampler, then dubbed to a worn tape.",
            values: build(&[
                (Saw, 0.0), (Pulse, 0.5), (PulseWidth, 0.35), (FmLevel, 0.7), (FmRatio, R14), (FmIndex, 1.6),
                (FmDecay, 0.25),
                (Cutoff, 2600.0), (Resonance, 0.1), (FilterEnv, 0.35), (KeyTrack, 0.5), (FilterVel, 0.4),
                (FAttack, 0.002), (FDecay, 0.6), (FSustain, 0.15), (FRelease, 0.3),
                (AAttack, 0.002), (ADecay, 2.2), (ASustain, 0.0), (ARelease, 0.35), (AmpVel, 0.5),
                (Chorus, 1.0), (RevDecay, 2.2), (RevMix, 0.25),
                (Wow, 0.45), (Flutter, 0.4), (Saturation, 0.45), (Age, 0.5), (Hiss, 0.35), (Crush, 0.35),
                (Volume, -1.0),
            ]),
        },
        Preset {
            name: "Sunset Bass",
            blurb: "Sub-heavy round bass with a soft pluck. Play it low.",
            values: build(&[
                (Saw, 0.55), (Sub, 0.9), (Pulse, 0.0),
                (Cutoff, 380.0), (Resonance, 0.2), (FilterEnv, 0.38), (KeyTrack, 0.3), (FilterVel, 0.3),
                (FAttack, 0.003), (FDecay, 0.32), (FSustain, 0.2), (FRelease, 0.2),
                (AAttack, 0.003), (ADecay, 0.7), (ASustain, 0.7), (ARelease, 0.18), (AmpVel, 0.4),
                (Glide, 0.12), (Chorus, 0.0), (RevMix, 0.08), (Saturation, 0.3), (Volume, -1.0),
            ]),
        },
        // Sound pack 1: mid-80s drum-machine synth-pop. Bright, chorused, sequenced.
        Preset {
            name: "Glasshouse Poly",
            blurb: "Two wide-detuned saws and a thin pulse through chorus II. Bright, glassy chords that sit on the beat.",
            values: build(&[
                (Saw, 0.8), (Detune, 14.0), (Pulse, 0.35), (PulseWidth, 0.3), (Pwm, 0.2),
                (Cutoff, 3400.0), (Resonance, 0.15), (FilterEnv, 0.25), (KeyTrack, 0.5), (FilterVel, 0.25),
                (FAttack, 0.004), (FDecay, 1.2), (FSustain, 0.4), (FRelease, 0.8),
                (AAttack, 0.015), (ADecay, 2.0), (ASustain, 0.8), (ARelease, 0.9), (AmpVel, 0.3),
                (LfoRate, 0.7), (Chorus, 2.0),
                (RevDecay, 2.8), (RevDamp, 0.35), (RevPre, 20.0), (RevMix, 0.22), (Volume, -7.0),
            ]),
        },
        Preset {
            name: "Night Bus Seq",
            blurb: "Short resonant pluck for 16th-note sequences. Accents open the filter, so velocity is the groove.",
            values: build(&[
                (Saw, 0.6), (Detune, 5.0), (Pulse, 0.7), (PulseWidth, 0.25),
                (Cutoff, 900.0), (Resonance, 0.35), (FilterEnv, 0.55), (KeyTrack, 0.6), (FilterVel, 0.5),
                (FAttack, 0.001), (FDecay, 0.16), (FSustain, 0.0), (FRelease, 0.12),
                (AAttack, 0.001), (ADecay, 0.3), (ASustain, 0.0), (ARelease, 0.12), (AmpVel, 0.45),
                (Chorus, 1.0), (RevDecay, 1.6), (RevPre, 10.0), (RevMix, 0.15), (Volume, 3.0),
            ]),
        },
        Preset {
            name: "Towpath Bass",
            blurb: "A bass voice built to play up high as the tune: bright saw, chorus I, a bit of bite. Try it an octave up.",
            values: build(&[
                (Saw, 0.9), (Pulse, 0.3), (PulseWidth, 0.35), (Sub, 0.15), (Detune, 3.0),
                (Cutoff, 1100.0), (Resonance, 0.25), (FilterEnv, 0.45), (KeyTrack, 0.4), (FilterVel, 0.35),
                (FAttack, 0.002), (FDecay, 0.35), (FSustain, 0.3), (FRelease, 0.15),
                (AAttack, 0.002), (ADecay, 0.6), (ASustain, 0.65), (ARelease, 0.15), (AmpVel, 0.4),
                (Chorus, 1.0), (RevDecay, 1.5), (RevMix, 0.1), (Saturation, 0.2), (Volume, -2.0),
            ]),
        },
        Preset {
            name: "Redbrick Strings",
            blurb: "String-machine ensemble: wide saws, slow PWM and the I+II shimmer. Swells in, hangs in the room.",
            values: build(&[
                (Saw, 0.9), (Detune, 18.0), (Pulse, 0.25), (PulseWidth, 0.4), (Pwm, 0.5),
                (Cutoff, 3000.0), (Resonance, 0.05), (FilterEnv, 0.1), (KeyTrack, 0.5), (FilterVel, 0.1),
                (FAttack, 0.4), (FDecay, 2.0), (FSustain, 0.7), (FRelease, 1.5),
                (AAttack, 0.35), (ADecay, 1.0), (ASustain, 0.92), (ARelease, 1.5), (AmpVel, 0.2),
                (LfoRate, 5.5), (LfoDelay, 0.6), (Vibrato, 0.04), (Chorus, 3.0),
                (RevDecay, 3.5), (RevDamp, 0.4), (RevPre, 30.0), (RevMix, 0.3), (Volume, -7.0),
            ]),
        },
        Preset {
            name: "Overcast Brass",
            blurb: "Hard, bright poly stab: detuned saws, a fast filter blip and a little drive. Play it tight.",
            values: build(&[
                (Saw, 1.0), (Detune, 10.0), (Pulse, 0.2), (PulseWidth, 0.25),
                (Cutoff, 700.0), (Resonance, 0.15), (FilterEnv, 0.7), (KeyTrack, 0.5), (FilterVel, 0.4),
                (FAttack, 0.015), (FDecay, 0.22), (FSustain, 0.25), (FRelease, 0.15),
                (AAttack, 0.004), (ADecay, 0.3), (ASustain, 0.55), (ARelease, 0.15), (AmpVel, 0.4),
                (Chorus, 1.0), (RevDecay, 1.8), (RevMix, 0.16), (Saturation, 0.3), (Volume, -4.0),
            ]),
        },
        Preset {
            name: "Platform Bell",
            blurb: "Digital FM bell at 7:1 over a thin pulse. A clean, cold strike for countermelodies.",
            values: build(&[
                (Saw, 0.0), (Pulse, 0.2), (PulseWidth, 0.2), (FmLevel, 0.85), (FmRatio, R7), (FmIndex, 2.4),
                (FmDecay, 0.4),
                (Cutoff, 9000.0), (Resonance, 0.0), (FilterEnv, 0.1), (KeyTrack, 0.2), (FilterVel, 0.3),
                (FAttack, 0.001), (FDecay, 0.8), (FSustain, 0.0), (FRelease, 0.6),
                (AAttack, 0.001), (ADecay, 1.8), (ASustain, 0.0), (ARelease, 0.8), (AmpVel, 0.5),
                (Chorus, 1.0), (RevDecay, 2.5), (RevPre, 30.0), (RevMix, 0.25), (Volume, -3.0),
            ]),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_in_range_and_named_uniquely() {
        let ps = all();
        assert!(ps.len() >= 8);
        let mut names = std::collections::HashSet::new();
        for p in &ps {
            assert!(names.insert(p.name));
            for &id in params::ALL {
                let d = id.def();
                let v = p.values[id as usize];
                assert!(v >= d.min && v <= d.max, "{} {:?} = {v}", p.name, id);
            }
        }
    }
}
