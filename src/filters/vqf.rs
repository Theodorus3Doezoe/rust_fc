use core::time::Duration;

use nalgebra::Vector3;
use uf_ahrs::{Ahrs, Vqf, VqfParams};

use crate::types::ImuBurst;

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

    pub fn update(&mut self, data: ImuBurst) -> nalgebra::UnitQuaternion<f32> {
        let gyro_rad = Vector3::new(
            data.gyro.x.to_radians(),
            data.gyro.y.to_radians(),
            data.gyro.z.to_radians(),
        );

        let accel_ms2 = Vector3::new(data.accel.x, data.accel.y, data.accel.z) * 9.81;

        // no mag implemented yet
        let mag = Vector3::new(0.0, 0.0, 0.0);

        self.vqf.update(gyro_rad, accel_ms2, mag);

        self.vqf.orientation()
    }

    pub fn orientation(&self) -> nalgebra::UnitQuaternion<f32> {
        self.vqf.orientation()
    }
}
