use biquad::*;

pub struct DtermFilter {
    d_term: DirectForm2Transposed<f32>,
    prev_d_filtered: f32,
    alpha: f32,
}

impl DtermFilter {
    pub fn new(sample_rate_hz: f32, cutoff_lpf1_hz: f32, cutoff_lpf2_hz: f32) -> Self {
        let coeffs = Coefficients::<f32>::from_params(
            Type::LowPass,
            sample_rate_hz.hz(),
            cutoff_lpf2_hz.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();
        let dt = 1.0 / sample_rate_hz;
        let alpha = 2.0 * core::f32::consts::PI * cutoff_lpf1_hz * dt;

        Self {
            d_term: DirectForm2Transposed::<f32>::new(coeffs),
            prev_d_filtered: 0.0,
            alpha,
        }
    }

    pub fn apply(&mut self, raw_dterm: f32) -> f32 {
        let d_filtered = self.prev_d_filtered + self.alpha * (raw_dterm - self.prev_d_filtered);
        self.prev_d_filtered = d_filtered;
        self.d_term.run(d_filtered)
    }
}
