use crate::imu::{Imu, ImuAccel, ImuData, ImuGyro};
use embedded_hal_async::spi::SpiDevice;

const CONFIG: u8 = 0x1A;
const GYRO_CONFIG: u8 = 0x1B;
const ACCEL_CONFIG: u8 = 0x1C;
const ACCEL_GYRO_START: u8 = 0x3B | 0x80;

pub struct RawImuData {
    pub accel_x: i16,
    pub accel_y: i16,
    pub accel_z: i16,
    pub temp: i16,
    pub gyro_x: i16,
    pub gyro_y: i16,
    pub gyro_z: i16,
}

impl RawImuData {
    pub fn convert(self) -> ImuData {
        const ACCEL_SENSITIVITY: f32 = 8192.0;
        const GYRO_SENSITIVITY: f32 = 65.5;

        ImuData {
            accel: ImuAccel {
                x_g: self.accel_x as f32 / ACCEL_SENSITIVITY,
                y_g: self.accel_y as f32 / ACCEL_SENSITIVITY,
                z_g: self.accel_z as f32 / ACCEL_SENSITIVITY,
            },
            gyro: ImuGyro {
                x_dps: self.gyro_x as f32 / GYRO_SENSITIVITY,
                y_dps: self.gyro_y as f32 / GYRO_SENSITIVITY,
                z_dps: self.gyro_z as f32 / GYRO_SENSITIVITY,
            },
        }
    }
}

pub struct Mpu6500<SPI: SpiDevice> {
    spi: SPI,
}

impl<SPI: SpiDevice> Mpu6500<SPI> {
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }

    pub async fn init_registers(&mut self) -> Result<(), SPI::Error> {
        defmt::info!("writing config");
        self.spi.write(&[CONFIG, 0b0000_0000]).await?;
        defmt::info!("writing gyro");
        self.spi.write(&[GYRO_CONFIG, 0b0000_1011]).await?;
        defmt::info!("writing accel");
        self.spi.write(&[ACCEL_CONFIG, 0b0000_1000]).await?;
        defmt::info!("done writing registers");
        Ok(())
    }

    pub async fn read_burst(&mut self) -> Result<RawImuData, SPI::Error> {
        let mut buf = [0u8; 15];
        buf[0] = ACCEL_GYRO_START;
        self.spi.transfer_in_place(&mut buf).await?;

        Ok(RawImuData {
            accel_x: i16::from_be_bytes([buf[1], buf[2]]),
            accel_y: i16::from_be_bytes([buf[3], buf[4]]),
            accel_z: i16::from_be_bytes([buf[5], buf[6]]),
            temp: i16::from_be_bytes([buf[7], buf[8]]),
            gyro_x: i16::from_be_bytes([buf[9], buf[10]]),
            gyro_y: i16::from_be_bytes([buf[11], buf[12]]),
            gyro_z: i16::from_be_bytes([buf[13], buf[14]]),
        })
    }
}

impl<SPI: SpiDevice> Imu for Mpu6500<SPI> {
    type Error = SPI::Error;
    type SpiBus = SPI;

    async fn init(&mut self) -> Result<(), Self::Error> {
        self.init_registers().await
    }

    async fn read(&mut self) -> Result<ImuData, Self::Error> {
        let raw = self.read_burst().await?;
        Ok(raw.convert())
    }

    fn spi_device_mut(&mut self) -> &mut Self::SpiBus {
        &mut self.spi
    }
}
