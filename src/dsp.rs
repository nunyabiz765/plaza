//! Small DSP building blocks shared by the voices and the effects.

use std::f32::consts::PI;

pub const TAU: f32 = 2.0 * PI;

/// PolyBLEP residual: subtract from a naive saw (or add/subtract at each edge of
/// a pulse) to round off the discontinuity and push aliasing well down.
#[inline]
pub fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// Band-limited sawtooth, phase in [0, 1).
#[inline]
pub fn saw(t: f32, dt: f32) -> f32 {
    2.0 * t - 1.0 - poly_blep(t, dt)
}

/// Band-limited pulse with duty `pw` in (0, 1).
#[inline]
pub fn pulse(t: f32, dt: f32, pw: f32) -> f32 {
    let mut y = if t < pw { 1.0 } else { -1.0 };
    y += poly_blep(t, dt);
    let mut t2 = t - pw;
    if t2 < 0.0 {
        t2 += 1.0;
    }
    y -= poly_blep(t2, dt);
    // Keep the average at zero whatever the width, so PWM doesn't thump.
    y - (2.0 * pw - 1.0)
}

/// Smooth saturator: matches tanh closely up to |x| = 3 and is exactly ±1 beyond.
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    if x <= -3.0 {
        -1.0
    } else if x >= 3.0 {
        1.0
    } else {
        let x2 = x * x;
        (x * (27.0 + x2) / (27.0 + 9.0 * x2)).clamp(-1.0, 1.0)
    }
}

/// Fast sine for phase in [0, 1). Parabolic approximation with one refinement
/// step; error well under 0.1%, which is inaudible on an LFO or FM operator.
#[inline]
pub fn sin01(t: f32) -> f32 {
    let x = t * TAU;
    let x = if x > PI { x - TAU } else { x };
    let y = (4.0 / PI) * x - (4.0 / (PI * PI)) * x * x.abs();
    0.225 * (y * y.abs() - y) + y
}

#[inline]
pub fn wrap01(t: f32) -> f32 {
    t - t.floor()
}

pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

pub fn midi_to_hz(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

/// xorshift32 noise. Deterministic per seed, so renders are reproducible.
#[derive(Clone)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(seed.max(1))
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// Uniform in [-1, 1).
    #[inline]
    pub fn bipolar(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (2.0 / 16_777_216.0) - 1.0
    }
}

/// One-pole low-pass, coefficient from a cutoff in Hz.
#[derive(Clone, Default)]
pub struct OnePole {
    a: f32,
    pub z: f32,
}

impl OnePole {
    pub fn new(hz: f32, sr: f32) -> Self {
        let mut f = OnePole::default();
        f.set(hz, sr);
        f
    }
    pub fn set(&mut self, hz: f32, sr: f32) {
        self.a = 1.0 - (-TAU * hz / sr).exp();
    }
    #[inline]
    pub fn lp(&mut self, x: f32) -> f32 {
        self.z += self.a * (x - self.z);
        self.z
    }
    #[inline]
    pub fn hp(&mut self, x: f32) -> f32 {
        x - self.lp(x)
    }
}

/// DC blocker (leaky differentiator, ~10 Hz corner at 48 kHz).
#[derive(Clone, Default)]
pub struct DcBlock {
    x1: f32,
    y1: f32,
    r: f32,
}

impl DcBlock {
    pub fn new(sr: f32) -> Self {
        DcBlock { x1: 0.0, y1: 0.0, r: 1.0 - TAU * 10.0 / sr }
    }
    #[inline]
    pub fn run(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + self.r * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}

/// Topology-preserving (trapezoidal) state-variable filter, used for fixed tone
/// shaping in the effects. Stable under modulation.
#[derive(Clone, Default)]
pub struct Svf {
    g: f32,
    k: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    ic1: f32,
    ic2: f32,
}

pub struct SvfOut {
    pub lp: f32,
    pub bp: f32,
    pub hp: f32,
}

impl Svf {
    pub fn new(hz: f32, q: f32, sr: f32) -> Self {
        let mut f = Svf::default();
        f.set(hz, q, sr);
        f
    }
    pub fn set(&mut self, hz: f32, q: f32, sr: f32) {
        let hz = hz.clamp(10.0, sr * 0.49);
        self.g = (PI * hz / sr).tan();
        self.k = 1.0 / q;
        self.a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        self.a2 = self.g * self.a1;
        self.a3 = self.g * self.a2;
    }
    #[inline]
    pub fn run(&mut self, x: f32) -> SvfOut {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        SvfOut { lp: v2, bp: v1, hp: x - self.k * v1 - v2 }
    }
    #[inline]
    pub fn lp(&mut self, x: f32) -> f32 {
        self.run(x).lp
    }
    #[inline]
    pub fn hp(&mut self, x: f32) -> f32 {
        self.run(x).hp
    }
}

/// Four-pole transistor-ladder-style low-pass (Zavalishin's zero-delay-feedback
/// form) with a saturating input stage. `k` is feedback, 0..4; at 4 it rings.
#[derive(Clone, Default)]
pub struct Ladder {
    s: [f32; 4],
}

impl Ladder {
    #[inline]
    pub fn run(&mut self, x: f32, cutoff: f32, k: f32, sr: f32) -> f32 {
        let fc = cutoff.clamp(16.0, sr * 0.45);
        let g = (PI * fc / sr).tan();
        let a = g / (1.0 + g);
        let b = 1.0 / (1.0 + g);
        let a2 = a * a;
        let sigma = (a2 * a * self.s[0] + a2 * self.s[1] + a * self.s[2] + self.s[3]) * b;
        // Loses some low end as resonance rises, like the real thing; give back half of it.
        let u = soft_clip((x * (1.0 + 0.5 * k) - k * sigma) / (1.0 + k * a2 * a2));
        let mut y = u;
        for s in &mut self.s {
            let v = (y - *s) * a;
            let out = v + *s;
            *s = out + v;
            y = out;
        }
        y
    }
    pub fn reset(&mut self) {
        self.s = [0.0; 4];
    }
}

/// Analog-flavoured ADSR. Attack aims past 1.0 so the curve is the familiar
/// rounded RC charge; decay and release are exponential.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Idle,
    Attack,
    Decay,
    Release,
}

#[derive(Clone)]
pub struct Adsr {
    pub level: f32,
    pub stage: Stage,
}

impl Default for Adsr {
    fn default() -> Self {
        Adsr { level: 0.0, stage: Stage::Idle }
    }
}

const ATTACK_TARGET: f32 = 1.3;
// ln(1 / (1 - 1/1.3)): attack reaches 1.0 at exactly the set time.
const ATTACK_TAUS: f32 = 1.466_337;
// Decay/release reach about -40 dB at the set time.
const FALL_TAUS: f32 = 4.6;

#[inline]
fn coef(time: f32, taus: f32, sr: f32) -> f32 {
    1.0 - (-taus / (time.max(0.0005) * sr)).exp()
}

impl Adsr {
    pub fn gate_on(&mut self) {
        // Retrigger from wherever we are: no click on fast repeats or voice steals.
        self.stage = Stage::Attack;
    }
    pub fn gate_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }
    /// Per-sample step. Times in seconds, already scaled for slow mode.
    #[inline]
    pub fn next(&mut self, a: f32, d: f32, s: f32, r: f32, sr: f32) -> f32 {
        match self.stage {
            Stage::Idle => {}
            Stage::Attack => {
                self.level += coef(a, ATTACK_TAUS, sr) * (ATTACK_TARGET - self.level);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => self.level += coef(d, FALL_TAUS, sr) * (s - self.level),
            Stage::Release => {
                self.level += coef(r, FALL_TAUS, sr) * (0.0 - self.level);
                if self.level < 1e-5 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
    pub fn idle(&self) -> bool {
        self.stage == Stage::Idle
    }
}

/// Circular delay line with 4-point Hermite interpolation for modulated reads.
#[derive(Clone)]
pub struct DelayLine {
    buf: Vec<f32>,
    pos: usize,
    mask: usize,
}

impl DelayLine {
    pub fn new(max_samples: usize) -> Self {
        let n = (max_samples + 4).next_power_of_two();
        DelayLine { buf: vec![0.0; n], pos: 0, mask: n - 1 }
    }
    #[inline]
    pub fn push(&mut self, x: f32) {
        self.pos = (self.pos + 1) & self.mask;
        self.buf[self.pos] = x;
    }
    /// Integer tap, `d` samples ago (d >= 0).
    #[inline]
    pub fn tap(&self, d: usize) -> f32 {
        self.buf[(self.pos.wrapping_sub(d)) & self.mask]
    }
    /// Fractional tap, `d` samples ago (d >= 1).
    #[inline]
    pub fn read(&self, d: f32) -> f32 {
        let d = d.max(1.0);
        let i = d as usize;
        let f = d - i as f32;
        let xm1 = self.tap(i - 1);
        let x0 = self.tap(i);
        let x1 = self.tap(i + 1);
        let x2 = self.tap(i + 2);
        let c1 = 0.5 * (x1 - xm1);
        let c2 = xm1 - 2.5 * x0 + 2.0 * x1 - 0.5 * x2;
        let c3 = 0.5 * (x2 - xm1) + 1.5 * (x0 - x1);
        ((c3 * f + c2) * f + c1) * f + x0
    }
    pub fn clear(&mut self) {
        self.buf.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_clip_is_bounded_and_odd() {
        for i in -1000..=1000 {
            let x = i as f32 * 0.01;
            let y = soft_clip(x);
            assert!(y.abs() <= 1.0);
            assert!((y + soft_clip(-x)).abs() < 1e-6);
        }
        assert!((soft_clip(0.5) - 0.5f32.tanh()).abs() < 0.01);
    }

    #[test]
    fn fast_sine_is_close() {
        for i in 0..1000 {
            let t = i as f32 / 1000.0;
            assert!((sin01(t) - (t * TAU).sin()).abs() < 0.002, "{t}");
        }
    }

    #[test]
    fn adsr_attack_hits_one_on_time() {
        let sr = 48_000.0;
        let mut e = Adsr::default();
        e.gate_on();
        let mut n = 0;
        while e.stage == Stage::Attack {
            e.next(0.1, 1.0, 0.5, 1.0, sr);
            n += 1;
        }
        let t = n as f32 / sr;
        assert!((t - 0.1).abs() < 0.002, "attack took {t}");
        for _ in 0..(sr as usize * 3) {
            e.next(0.1, 0.3, 0.5, 1.0, sr);
        }
        assert!((e.level - 0.5).abs() < 0.01);
        e.gate_off();
        for _ in 0..(sr as usize * 2) {
            e.next(0.1, 0.3, 0.5, 0.2, sr);
        }
        assert!(e.idle());
    }

    #[test]
    fn hermite_reads_between_samples() {
        let mut d = DelayLine::new(64);
        for i in 0..32 {
            d.push(i as f32);
        }
        // Last pushed is 31, so 2.5 samples ago is 28.5 on a ramp.
        assert!((d.read(2.5) - 28.5).abs() < 1e-4);
    }

    #[test]
    fn ladder_is_stable_at_full_resonance() {
        let sr = 48_000.0;
        let mut f = Ladder::default();
        let mut rng = Rng::new(7);
        let mut peak = 0f32;
        for i in 0..sr as usize {
            let cutoff = 200.0 + 15_000.0 * (0.5 + 0.5 * (i as f32 * 0.0007).sin());
            let y = f.run(rng.bipolar() * 2.0, cutoff, 4.0, sr);
            assert!(y.is_finite());
            peak = peak.max(y.abs());
        }
        assert!(peak < 4.0, "peak {peak}");
    }

    #[test]
    fn ladder_cuts_highs_and_passes_lows() {
        let sr = 48_000.0;
        let rms = |freq: f32| {
            let mut f = Ladder::default();
            let mut acc = 0.0;
            for i in 0..48_000 {
                let x = 0.1 * (TAU * freq * i as f32 / sr).sin();
                let y = f.run(x, 1000.0, 0.0, sr);
                if i > 4800 {
                    acc += y * y;
                }
            }
            (acc / 43_200.0).sqrt()
        };
        let pass = rms(100.0);
        let stop = rms(8000.0);
        assert!(pass > 0.06, "pass {pass}");
        // 24 dB/oct: three octaves above cutoff should be ~72 dB down; insist on 50.
        assert!(20.0 * (stop / pass).log10() < -50.0, "stop {stop} pass {pass}");
    }
}
