use super::{Imu, ImuBurst};
use embedded_hal_async::spi::SpiDevice;

use crate::types::Vector3D;

pub struct CalibratedImu<I> {
    inner_imu: I,
    gyro_offset: Vector3D,
}

impl<I> CalibratedImu<I> {
    pub fn new(inner_imu: I) -> Self {
        Self {
            inner_imu,
            gyro_offset: Vector3D {
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

            sum_x += gyro.x;
            sum_y += gyro.y;
            sum_z += gyro.z;

            if i.is_multiple_of(50) {
                defmt::info!("Gyro read count: {} value: {}", i, gyro);
            }
            embassy_time::Timer::after_micros(1000).await;
        }
        defmt::info!("sum_x: {}. sum_y: {}, sum_z: {}", sum_x, sum_y, sum_z);

        self.gyro_offset = Vector3D {
            x: sum_x / SAMPLES as f32,
            y: sum_y / SAMPLES as f32,
            z: sum_z / SAMPLES as f32,
        };

        defmt::info!("Kalibratie successful! Offset: {}", self.gyro_offset);
        Ok(())
    }
}

impl<I, SPI> Imu<SPI> for CalibratedImu<I>
where
    I: Imu<SPI>,
    SPI: SpiDevice,
{
    async fn read_accel(&mut self) -> Result<Vector3D, SPI::Error> {
        self.inner_imu.read_accel().await
    }

    async fn read_gyro(&mut self) -> Result<Vector3D, SPI::Error> {
        let raw = self.inner_imu.read_gyro().await?;
        Ok(Vector3D {
            x: raw.x - self.gyro_offset.x,
            y: raw.y - self.gyro_offset.y,
            z: raw.z - self.gyro_offset.z,
        })
    }

    async fn read_burst(&mut self) -> Result<ImuBurst, SPI::Error> {
        let mut burst = self.inner_imu.read_burst().await?;
        // Trek automatisch de offset af van de gyro
        burst.gyro.x -= self.gyro_offset.x;
        burst.gyro.y -= self.gyro_offset.y;
        burst.gyro.z -= self.gyro_offset.z;
        Ok(burst)
    }
}
