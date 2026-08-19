use biquad::*;

use crate::types::Rates;

pub struct GyroFilter {
    roll: DirectForm2Transposed<f32>,
    pitch: DirectForm2Transposed<f32>,
    yaw: DirectForm2Transposed<f32>,
}

impl GyroFilter {
    pub fn new(sample_rate_hz: f32, cutoff_hz: f32) -> Self {
        let coeffs = Coefficients::<f32>::from_params(
            Type::LowPass,
            sample_rate_hz.hz(),
            cutoff_hz.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();

        Self {
            roll: DirectForm2Transposed::<f32>::new(coeffs),
            pitch: DirectForm2Transposed::<f32>::new(coeffs),
            yaw: DirectForm2Transposed::<f32>::new(coeffs),
        }
    }

    pub fn apply(&mut self, gyro: Rates) -> Rates {
        Rates {
            roll: self.roll.run(gyro.roll),
            pitch: self.pitch.run(gyro.pitch),
            yaw: self.yaw.run(gyro.yaw),
        }
    }
}
