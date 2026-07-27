use crate::imu::{Imu, ImuAccel, ImuData, ImuGyro};
use embassy_time::Timer;

#[derive(defmt::Format)]
pub struct ImuBias {
    gyro_x_dps: f32,
    gyro_y_dps: f32,
    gyro_z_dps: f32,
    accel_x_g: f32,
    accel_y_g: f32,
    accel_z_g: f32,
}

impl ImuBias {
    pub fn apply(&self, raw: ImuData) -> ImuData {
        ImuData {
            accel: ImuAccel {
                x_g: raw.accel.x_g - self.accel_x_g,
                y_g: raw.accel.y_g - self.accel_y_g,
                z_g: raw.accel.z_g - (self.accel_z_g - 1.0),
            },
            gyro: ImuGyro {
                x_dps: raw.gyro.x_dps - self.gyro_x_dps,
                y_dps: raw.gyro.y_dps - self.gyro_y_dps,
                z_dps: raw.gyro.z_dps - self.gyro_z_dps,
            },
        }
    }
}

pub async fn calibrate<T: Imu>(imu: &mut T, samples: u32) -> ImuBias {
    defmt::info!("Calibrating, keep sensor still...");
    let mut sum_ax = 0.0;
    let mut sum_ay = 0.0;
    let mut sum_az = 0.0;
    let mut sum_gx = 0.0;
    let mut sum_gy = 0.0;
    let mut sum_gz = 0.0;

    let mut n: f32 = 0.0;

    for _ in 0..samples {
        match imu.read().await {
            Ok(data) => {
                sum_ax += data.accel.x_g;
                sum_ay += data.accel.y_g;
                sum_az += data.accel.z_g;
                sum_gx += data.gyro.x_dps;
                sum_gy += data.gyro.y_dps;
                sum_gz += data.gyro.z_dps;
                n += 1.0;
            }
            Err(_) => {
                defmt::warn!("Reading imu during calibration failed");
            }
        }
        Timer::after_micros(500).await;
    }
    defmt::info!("Calibrating finished");

    ImuBias {
        accel_x_g: sum_ax / n,
        accel_y_g: sum_ay / n,
        accel_z_g: sum_az / n,
        gyro_x_dps: sum_gx / n,
        gyro_y_dps: sum_gy / n,
        gyro_z_dps: sum_gz / n,
    }
}
