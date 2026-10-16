use biquad::{Biquad, Coefficients, DirectForm1, Hertz, ToHertz, Type};

pub struct Notch {
    fs: f32,
    q: f32,
    f0: f32,
    biquad: DirectForm1<f32>,
}

impl Notch {
    pub fn new(fs: f32, f0: f32, q: f32) -> Self {
        let coeffs = Coefficients::from_params(Type::Notch, fs.hz(), f0.hz(), q).unwrap();
        let biquad = DirectForm1::new(coeffs);
        Self { fs, q, biquad, f0 }
    }

    pub fn set_frequency(&mut self, f0: f32) {
        if f0 <= 1.0 {
            return;
        }
        let f0 = f0.min(self.fs * 0.45);
        let diff = ((f0 - self.f0) / self.f0 * 100.0).abs();
        if diff > 5.0 {
            let new_coeffs =
                Coefficients::from_params(Type::Notch, self.fs.hz(), f0.hz(), self.q).unwrap();
            self.biquad.update_coefficients(new_coeffs);
            self.f0 = f0;
        }
    }

    pub fn apply(&mut self, x: f32) -> f32 {
        self.biquad.run(x)
    }
}
