use core::time::Duration;

use crate::imu::ImuData;
use nalgebra::Vector3;
use uf_ahrs::{Ahrs, Vqf, VqfParams};

pub struct VqfFilter {
    vqf: Vqf,
    dt: Duration,
}

impl VqfFilter {
    pub fn new(dt: Duration) -> Self {
        let params = VqfParams::default();
        let vqf = Vqf::new(dt, params);
        Self { vqf, dt }
    }

    pub fn update(&mut self, data: &ImuData) -> nalgebra::UnitQuaternion<f32> {
        let gyro_rad = Vector3::new(
            data.gyro.x_dps * (core::f32::consts::PI / 180.0),
            data.gyro.y_dps * (core::f32::consts::PI / 180.0),
            data.gyro.z_dps * (core::f32::consts::PI / 180.0),
        );

        let accel_ms2 = Vector3::new(
            data.accel.x_g * 9.81,
            data.accel.y_g * 9.81,
            data.accel.z_g * 9.81,
        );

        let mag = Vector3::new(0.0, 0.0, 0.0);

        self.vqf.update(gyro_rad, accel_ms2, mag);

        self.vqf.orientation()
    }

    pub fn orientation(&self) -> nalgebra::UnitQuaternion<f32> {
        self.vqf.orientation()
    }
}
