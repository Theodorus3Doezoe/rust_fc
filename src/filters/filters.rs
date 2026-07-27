use crate::imu::ImuGyro;
use biquad::*;

pub struct GyroFilter {
    x: DirectForm2Transposed<f32>,
    y: DirectForm2Transposed<f32>,
    z: DirectForm2Transposed<f32>,
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
            x: DirectForm2Transposed::<f32>::new(coeffs),
            y: DirectForm2Transposed::<f32>::new(coeffs),
            z: DirectForm2Transposed::<f32>::new(coeffs),
        }
    }

    pub fn apply(&mut self, gyro: ImuGyro) -> ImuGyro {
        ImuGyro {
            x_dps: self.x.run(gyro.x_dps),
            y_dps: self.y.run(gyro.y_dps),
            z_dps: self.z.run(gyro.z_dps),
        }
    }
}
