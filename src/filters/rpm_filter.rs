use crate::types::Rates;

use super::notch::Notch;
use core::array::from_fn;
pub struct RpmFilter<const SLOTS: usize> {
    notches: [[Notch; SLOTS]; 3],
}

pub const HARMONICS: usize = 2;

impl<const SLOTS: usize> RpmFilter<SLOTS> {
    pub fn new(fs: f32) -> Self {
        Self {
            notches: from_fn(|_axis| from_fn(|_slot| Notch::new(fs, 100.0, 3.0))),
        }
    }

    pub fn update<const N: usize>(&mut self, freqs: &[Option<f32>; N]) {
        for (motor, freq) in freqs.iter().enumerate() {
            let Some(base) = *freq else {
                continue;
            };

            for h in 0..HARMONICS {
                let f = base * (h + 1) as f32;
                for axis in 0..3 {
                    self.notches[axis][motor * HARMONICS + h].set_frequency(f);
                }
            }
        }
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
