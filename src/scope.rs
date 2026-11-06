//! Lock-free sample tap from the audio thread to the oscilloscope.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering::Relaxed};

pub struct Scope {
    buf: Vec<AtomicU32>,
    pos: AtomicUsize,
}

impl Scope {
    pub fn new(len: usize) -> Self {
        let len = len.next_power_of_two();
        Scope { buf: (0..len).map(|_| AtomicU32::new(0)).collect(), pos: AtomicUsize::new(0) }
    }

    /// Audio thread: append a mono sample.
    #[inline]
    pub fn push(&self, x: f32) {
        let p = self.pos.load(Relaxed);
        self.buf[p & (self.buf.len() - 1)].store(x.to_bits(), Relaxed);
        self.pos.store(p.wrapping_add(1), Relaxed);
    }

    /// UI thread: the most recent `n` samples, oldest first. Tearing at the
    /// write edge is harmless for a display.
    pub fn latest(&self, n: usize) -> Vec<f32> {
        let n = n.min(self.buf.len());
        let end = self.pos.load(Relaxed);
        let mask = self.buf.len() - 1;
        (0..n).map(|i| f32::from_bits(self.buf[end.wrapping_sub(n - i) & mask].load(Relaxed))).collect()
    }
}

/// Builds an SVG path for `w` x `h` from a window of samples, triggered on a
/// rising zero crossing so periodic waves stand still.
pub fn trace(samples: &[f32], points: usize, w: f32, h: f32) -> String {
    if samples.len() < points * 2 {
        return format!("M 0 {} L {w} {}", h / 2.0, h / 2.0);
    }
    let search = samples.len() - points;
    let start = (1..search).find(|&i| samples[i - 1] < 0.0 && samples[i] >= 0.0).unwrap_or(search);
    let win = &samples[start..start + points];
    let peak = win.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    // Gentle auto-gain so quiet pads still draw a readable shape.
    let gain = if peak > 1e-4 { (0.9 / peak).min(6.0) } else { 1.0 };
    let mut s = String::with_capacity(points * 14);
    for (i, &x) in win.iter().enumerate() {
        let px = i as f32 * w / (points - 1) as f32;
        let py = h / 2.0 - x.clamp(-1.0, 1.0) * gain * h * 0.45;
        s.push_str(if i == 0 { "M " } else { " L " });
        s.push_str(&format!("{px:.1} {py:.1}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_returns_latest_in_order() {
        let s = Scope::new(8);
        for i in 0..20 {
            s.push(i as f32);
        }
        assert_eq!(s.latest(4), vec![16.0, 17.0, 18.0, 19.0]);
    }

    #[test]
    fn trace_is_a_path() {
        let x: Vec<f32> = (0..2000).map(|i| (i as f32 * 0.05).sin()).collect();
        let p = trace(&x, 300, 600.0, 120.0);
        assert!(p.starts_with("M 0.0 "));
        assert_eq!(p.matches(" L ").count(), 299);
        assert!(trace(&[], 300, 600.0, 120.0).starts_with("M 0 60"));
    }
}
