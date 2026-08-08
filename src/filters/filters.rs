use crate::imu::ImuGyro;
use biquad::*;

pub struct GyroFilter {
    x: DirectForm2Transposed<f32>,
    y: DirectForm2Transposed<f32>,
    z: DirectForm2Transposed<f32>,
}

// test pt1 filter and higher cutoff frequencies if phase delay is to prominent.
// Later trusting mostly on rpm notch filter if possible to reduce delay
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

pub struct DtermFilter {
    d_term: DirectForm2Transposed<f32>,
    prev_d_filtered: f32,
    alpha: f32,
}

impl DtermFilter {
    pub fn new(sample_rate_hz: f32, cutoff_hz: f32) -> Self {
        let coeffs = Coefficients::<f32>::from_params(
            Type::LowPass,
            sample_rate_hz.hz(),
            cutoff_hz.hz(),
            Q_BUTTERWORTH_F32,
        )
        .unwrap();
        let dt = 1.0 / sample_rate_hz;
        let alpha = 2.0 * core::f32::consts::PI * cutoff_hz * dt;

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
