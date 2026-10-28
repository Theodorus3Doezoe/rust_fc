use biquad::{Biquad, Coefficients, DirectForm1};

pub struct Notch {
    biquad: DirectForm1<f32>,
}

impl Notch {
    pub fn from_coeffs(coeffs: Coefficients<f32>) -> Self {
        Self {
            biquad: DirectForm1::<f32>::new(coeffs),
        }
    }

    pub fn set_coeffs(&mut self, coeffs: Coefficients<f32>) {
        self.biquad.update_coefficients(coeffs);
    }

    pub fn apply(&mut self, x: f32) -> f32 {
        self.biquad.run(x)
    }
}
