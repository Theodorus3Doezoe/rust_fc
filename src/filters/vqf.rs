use core::time::Duration;

use nalgebra::Vector3;
use uf_ahrs::{Vqf, VqfParams};

use crate::types::ImuBurst;

pub struct VqfFilter {
    vqf: Vqf,
}

impl VqfFilter {
    pub fn new(dt: Duration) -> Self {
        let params = VqfParams {
            rest_threshold_gyro: 2.0_f32.to_radians(),
            ..VqfParams::default()
        };
        let vqf = Vqf::new(dt, params);
        Self { vqf }
    }

    pub fn update(&mut self, data: ImuBurst) -> nalgebra::UnitQuaternion<f32> {
        let gyro = Vector3::new(data.gyro.roll, data.gyro.pitch, data.gyro.yaw);
        let accel = Vector3::new(data.accel.x, data.accel.y, data.accel.z);

        self.vqf.update2(gyro, accel);

        self.vqf.orientation()
    }

    pub fn orientation(&self) -> nalgebra::UnitQuaternion<f32> {
        self.vqf.orientation()
    }

    pub fn is_rest(&self) -> bool {
        self.vqf.is_rest_phase()
    }
}
