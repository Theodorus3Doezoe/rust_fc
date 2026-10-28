use biquad::{Coefficients, ToHertz, Type};

use crate::types::Rates;

use super::notch::Notch;
use core::array::from_fn;
pub struct RpmFilter<const SLOTS: usize> {
    notches: [[Notch; SLOTS]; 3],
    fs: f32,
    q: f32,
    f0: [f32; SLOTS],
    next: usize,
}

pub const HARMONICS: usize = 2;

impl<const SLOTS: usize> RpmFilter<SLOTS> {
    pub fn new(fs: f32) -> Self {
        let q = 3.0;
        let coeffs = Coefficients::from_params(Type::Notch, fs.hz(), 100.0.hz(), q).unwrap();

        Self {
            notches: from_fn(|_| from_fn(|_| Notch::from_coeffs(coeffs))),
            fs,
            q,
            f0: [100.0; SLOTS],
            next: 0,
        }
    }

    pub fn update(&mut self, freqs: &[Option<f32>]) {
        let slot = self.next % SLOTS;
        self.next = self.next.wrapping_add(1);

        let motor = slot / HARMONICS;
        let h = slot % HARMONICS;
        let Some(base) = freqs.get(motor).copied().flatten() else {
            return;
        };

        let f = (base * (h + 1) as f32).min(self.fs * 0.45);
        if f <= 1.0 {
            return;
        }

        if ((f - self.f0[slot]) / self.f0[slot] * 100.0).abs() <= 5.0 {
            return;
        }

        let coeffs = Coefficients::from_params(Type::Notch, self.fs.hz(), f.hz(), self.q).unwrap();
        for axis in 0..3 {
            self.notches[axis][slot].set_coeffs(coeffs);
        }
        self.f0[slot] = f;
    }

    pub fn apply(&mut self, gyro: Rates) -> Rates {
        let mut out = gyro;
        for n in &mut self.notches[0] {
            out.roll = n.apply(out.roll);
        }
        for n in &mut self.notches[1] {
            out.pitch = n.apply(out.pitch);
        }
        for n in &mut self.notches[2] {
            out.yaw = n.apply(out.yaw);
        }
        out
    }
}
