use super::{Imu, ImuBurst};
use embedded_hal_async::spi::SpiDevice;

use crate::types::{Rates, Vec3};

pub struct CalibratedImu<I> {
    inner_imu: I,
    gyro_offset: Vec3,
}

impl<I> CalibratedImu<I> {
    pub fn new(inner_imu: I) -> Self {
        Self {
            inner_imu,
            gyro_offset: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        }
    }

    pub async fn calibrate<SPI: SpiDevice>(&mut self) -> Result<(), SPI::Error>
    where
        I: Imu<SPI>,
    {
        defmt::info!("Calibrating imu keep device still");
        embassy_time::Timer::after_millis(100).await;

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_z = 0.0;

        const SAMPLES: usize = 1000;

        for i in 0..SAMPLES {
            let gyro = self.inner_imu.read_gyro().await?;

            sum_x += gyro.roll;
            sum_y += gyro.pitch;
            sum_z += gyro.yaw;

            if i.is_multiple_of(50) {
                defmt::info!("Gyro read count: {} value: {}", i, gyro);
            }
            embassy_time::Timer::after_micros(1000).await;
        }
        defmt::info!("sum_x: {}. sum_y: {}, sum_z: {}", sum_x, sum_y, sum_z);

        self.gyro_offset = Vec3 {
            x: sum_x / SAMPLES as f32,
            y: sum_y / SAMPLES as f32,
            z: sum_z / SAMPLES as f32,
        };

        defmt::info!("Calibration sucessfull: {}", self.gyro_offset);
        Ok(())
    }
}

impl<I, SPI> Imu<SPI> for CalibratedImu<I>
where
    I: Imu<SPI>,
    SPI: SpiDevice,
{
    async fn read_accel(&mut self) -> Result<Vec3, SPI::Error> {
        let accel = self.inner_imu.read_accel().await?;
        Ok(Vec3 {
            x: accel.x,
            y: accel.y,
            z: accel.z,
        })
    }

    async fn read_gyro(&mut self) -> Result<Rates, SPI::Error> {
        let raw = self.inner_imu.read_gyro().await?;
        Ok(Rates {
            roll: raw.roll - self.gyro_offset.x,
            pitch: raw.pitch - self.gyro_offset.y,
            yaw: raw.yaw - self.gyro_offset.z,
        })
    }

    async fn read_burst(&mut self) -> Result<ImuBurst, SPI::Error> {
        let mut burst = self.inner_imu.read_burst().await?;
        burst.gyro.roll -= self.gyro_offset.x;
        burst.gyro.pitch -= self.gyro_offset.y;
        burst.gyro.yaw -= self.gyro_offset.z;
        Ok(burst)
    }
}
