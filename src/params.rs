//! Every sound parameter, its range and how a 0..1 knob maps onto it.
//!
//! The engine stores plain values (Hz, seconds, 0..1 levels). The UI works in
//! normalized knob positions; `to_norm` / `from_norm` convert between the two.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Curve {
    /// Straight line from min to max.
    Lin,
    /// Equal ratio per knob turn (frequencies, times). `min` must be > 0.
    Exp,
    /// Integer index into a list of choices, 0..=max.
    Steps,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Pct,
    Hz,
    Sec,
    Ms,
    Cents,
    Semis,
    Db,
    Ratio,
    Chorus,
    Rate,
}

pub struct ParamDef {
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub curve: Curve,
    pub unit: Unit,
}

const fn p(label: &'static str, min: f32, max: f32, default: f32, curve: Curve, unit: Unit) -> ParamDef {
    ParamDef { label, min, max, default, curve, unit }
}

/// FM modulator:carrier ratios on offer. Whole numbers sound harmonic (tines,
/// electric piano); the odd ones are where the bells live.
pub const FM_RATIOS: [f32; 11] = [0.5, 1.0, 1.5, 2.0, 3.0, 3.5, 4.0, 5.0, 7.0, 9.0, 14.0];
pub const CHORUS_MODES: [&str; 4] = ["Off", "I", "II", "I+II"];

macro_rules! params {
    ($( $id:ident => $def:expr ),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(usize)]
        pub enum P { $( $id ),* }
        pub const DEFS: &[ParamDef] = &[ $( $def ),* ];
        pub const ALL: &[P] = &[ $( P::$id ),* ];
    };
}

use Curve::*;
use Unit::*;

params! {
    // Oscillators
    Saw        => p("Saw", 0.0, 1.0, 0.8, Lin, Pct),
    Pulse      => p("Pulse", 0.0, 1.0, 0.0, Lin, Pct),
    PulseWidth => p("Width", 0.05, 0.5, 0.5, Lin, Pct),
    Pwm        => p("PWM", 0.0, 1.0, 0.0, Lin, Pct),
    Sub        => p("Sub", 0.0, 1.0, 0.0, Lin, Pct),
    Noise      => p("Noise", 0.0, 1.0, 0.0, Lin, Pct),
    Detune     => p("Detune", 0.0, 25.0, 0.0, Lin, Cents),
    Glide      => p("Glide", 0.0, 1.0, 0.0, Lin, Sec),
    // 2-op FM layer
    FmLevel    => p("FM", 0.0, 1.0, 0.0, Lin, Pct),
    FmRatio    => p("Ratio", 0.0, 10.0, 1.0, Steps, Ratio),
    FmIndex    => p("Index", 0.0, 8.0, 2.0, Lin, Ratio),
    FmDecay    => p("FM Decay", 0.02, 8.0, 0.8, Exp, Sec),
    // Filter
    Cutoff     => p("Cutoff", 30.0, 18000.0, 6000.0, Exp, Hz),
    Resonance  => p("Reso", 0.0, 1.0, 0.1, Lin, Pct),
    FilterEnv  => p("Env", 0.0, 1.0, 0.2, Lin, Pct),
    FilterLfo  => p("LFO", 0.0, 1.0, 0.0, Lin, Pct),
    KeyTrack   => p("Key", 0.0, 1.0, 0.5, Lin, Pct),
    FilterVel  => p("Vel", 0.0, 1.0, 0.3, Lin, Pct),
    // Filter envelope
    FAttack    => p("A", 0.001, 10.0, 0.005, Exp, Sec),
    FDecay     => p("D", 0.005, 15.0, 0.6, Exp, Sec),
    FSustain   => p("S", 0.0, 1.0, 0.3, Lin, Pct),
    FRelease   => p("R", 0.005, 15.0, 0.5, Exp, Sec),
    // Amp envelope
    AAttack    => p("A", 0.001, 10.0, 0.004, Exp, Sec),
    ADecay     => p("D", 0.005, 15.0, 0.8, Exp, Sec),
    ASustain   => p("S", 0.0, 1.0, 0.8, Lin, Pct),
    ARelease   => p("R", 0.005, 15.0, 0.4, Exp, Sec),
    AmpVel     => p("Vel", 0.0, 1.0, 0.5, Lin, Pct),
    // LFO
    LfoRate    => p("Rate", 0.05, 20.0, 4.0, Exp, Hz),
    LfoDelay   => p("Delay", 0.0, 4.0, 0.0, Lin, Sec),
    Vibrato    => p("Vibrato", 0.0, 1.0, 0.0, Lin, Pct),
    // Effects
    Chorus     => p("Chorus", 0.0, 3.0, 0.0, Steps, Unit::Chorus),
    RevDecay   => p("Decay", 0.5, 20.0, 3.5, Exp, Sec),
    RevDamp    => p("Damp", 0.0, 1.0, 0.4, Lin, Pct),
    RevPre     => p("Pre", 0.0, 200.0, 20.0, Lin, Ms),
    RevMix     => p("Mix", 0.0, 1.0, 0.25, Lin, Pct),
    Wow        => p("Wow", 0.0, 1.0, 0.0, Lin, Pct),
    Flutter    => p("Flutter", 0.0, 1.0, 0.0, Lin, Pct),
    Saturation => p("Sat", 0.0, 1.0, 0.0, Lin, Pct),
    Age        => p("Age", 0.0, 1.0, 0.0, Lin, Pct),
    Hiss       => p("Hiss", 0.0, 1.0, 0.0, Lin, Pct),
    Crush      => p("Lo-fi", 0.0, 1.0, 0.0, Lin, Pct),
    // Master
    Volume     => p("Volume", -30.0, 6.0, -6.0, Lin, Db),
    SlowRate   => p("Slow", 0.5, 0.95, 0.8, Lin, Rate),
}

pub const COUNT: usize = DEFS.len();
pub type Values = [f32; COUNT];

impl P {
    pub fn def(self) -> &'static ParamDef {
        &DEFS[self as usize]
    }
    pub fn from_index(i: usize) -> Option<P> {
        ALL.get(i).copied()
    }
    /// Discrete params snap instead of gliding.
    pub fn is_stepped(self) -> bool {
        self.def().curve == Steps
    }
}

pub fn defaults() -> Values {
    let mut v = [0.0; COUNT];
    for (i, d) in DEFS.iter().enumerate() {
        v[i] = d.default;
    }
    v
}

pub fn from_norm(p: P, n: f32) -> f32 {
    let d = p.def();
    let n = n.clamp(0.0, 1.0);
    match d.curve {
        Lin => d.min + n * (d.max - d.min),
        Exp => d.min * (d.max / d.min).powf(n),
        Steps => (d.min + n * (d.max - d.min)).round(),
    }
}

pub fn to_norm(p: P, v: f32) -> f32 {
    let d = p.def();
    let v = v.clamp(d.min, d.max);
    match d.curve {
        Lin | Steps => (v - d.min) / (d.max - d.min),
        Exp => (v / d.min).ln() / (d.max / d.min).ln(),
    }
}

/// Human-readable value for the knob readout.
pub fn display(p: P, v: f32) -> String {
    let d = p.def();
    match d.unit {
        Pct => format!("{:.0}%", v * 100.0),
        Hz if v >= 1000.0 => format!("{:.1} kHz", v / 1000.0),
        Hz if v < 10.0 => format!("{v:.2} Hz"),
        Hz => format!("{v:.0} Hz"),
        Sec if p == P::Glide => {
            if v <= 0.001 {
                "off".into()
            } else {
                format!("{:.0} ms", glide_seconds(v) * 1000.0)
            }
        }
        Sec if v < 1.0 => format!("{:.0} ms", v * 1000.0),
        Sec => format!("{v:.1} s"),
        Ms => format!("{v:.0} ms"),
        Cents => format!("{v:.0} ct"),
        Semis => format!("{v:+.1} st"),
        Db => format!("{v:+.0} dB"),
        Ratio if p == P::FmRatio => format!("×{}", trim(FM_RATIOS[(v as usize).min(FM_RATIOS.len() - 1)])),
        Ratio => format!("{v:.1}"),
        Unit::Chorus => CHORUS_MODES[(v as usize).min(3)].into(),
        Rate => format!("{:.1} st", 12.0 * v.log2()),
    }
}

fn trim(x: f32) -> String {
    if x.fract() == 0.0 {
        format!("{x:.0}")
    } else {
        format!("{x}")
    }
}

/// Glide knob is 0..1; the response is quadratic so the useful short times get
/// most of the travel. Full turn is 1.2 s.
pub fn glide_seconds(v: f32) -> f32 {
    v * v * 1.2
}

pub fn fm_ratio(v: f32) -> f32 {
    FM_RATIOS[(v.round() as usize).min(FM_RATIOS.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_round_trips() {
        for &p in ALL {
            for i in 0..=20 {
                let n = i as f32 / 20.0;
                let v = from_norm(p, n);
                let d = p.def();
                assert!(v >= d.min - 1e-4 && v <= d.max + 1e-3, "{p:?} {v}");
                if !p.is_stepped() {
                    assert!((to_norm(p, v) - n).abs() < 1e-3, "{p:?} at {n}");
                }
            }
        }
    }

    #[test]
    fn defaults_are_in_range() {
        for &p in ALL {
            let d = p.def();
            assert!(d.default >= d.min && d.default <= d.max, "{p:?}");
            if d.curve == Exp {
                assert!(d.min > 0.0);
            }
        }
        assert_eq!(DEFS.len(), ALL.len());
    }

    #[test]
    fn displays() {
        assert_eq!(display(P::Cutoff, 6000.0), "6.0 kHz");
        assert_eq!(display(P::Chorus, 2.0), "II");
        assert_eq!(display(P::FmRatio, 5.0), "×3.5");
        assert_eq!(display(P::SlowRate, 0.8), "-3.9 st");
        assert_eq!(display(P::Glide, 0.0), "off");
    }
}
