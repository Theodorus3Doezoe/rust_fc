#[derive(defmt::Format)]
struct ImuBias {
    gyro_x_dps: f32,
    gyro_y_dps: f32,
    gyro_z_dps: f32,
    accel_x_g: f32,
    accel_y_g: f32,
    accel_z_g: f32,
}

impl ImuBias {
    fn apply(&self, raw: ImuData) -> ImuData {
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

async fn calibrate(mpu: &mut Mpu6500, samples: u32) -> ImuBias {
    let mut sum_ax = 0.0;
    let mut sum_ay = 0.0;
    let mut sum_az = 0.0;
    let mut sum_gx = 0.0;
    let mut sum_gy = 0.0;
    let mut sum_gz = 0.0;

    let mut n: f32 = 0.0;

    for _ in 0..samples {
        let raw = mpu.read_burst().await;
        if let Some(raw) = raw {
            let imu = raw.convert();
            sum_ax += imu.accel.x_g;
            sum_ay += imu.accel.y_g;
            sum_az += imu.accel.z_g;
            sum_gx += imu.gyro.x_dps;
            sum_gy += imu.gyro.y_dps;
            sum_gz += imu.gyro.z_dps;
            n += 1.0;
        }
        Timer::after_micros(500).await
    }

    ImuBias {
        accel_x_g: sum_ax / n,
        accel_y_g: sum_ay / n,
        accel_z_g: sum_az / n,
        gyro_x_dps: sum_gx / n,
        gyro_y_dps: sum_gy / n,
        gyro_z_dps: sum_gz / n,
    }
}
