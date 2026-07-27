use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::SPI0;
use embassy_rp::spi::{Async, Error as SpiError, Spi};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embedded_hal_async::spi::SpiDevice as _;

const CONFIG: u8 = 0x1A;
const GYRO_CONFIG: u8 = 0x1B;
const ACCEL_CONFIG: u8 = 0x1C;
const ACCEL_GYRO_START: u8 = 0x3B | 0x80;

pub const BASE_CLOCK: u32 = 10_000_000;

#[derive(defmt::Format)]
pub struct ImuAccel {
    pub x_g: f32,
    pub y_g: f32,
    pub z_g: f32,
}

#[derive(defmt::Format)]
pub struct ImuGyro {
    pub x_dps: f32,
    pub y_dps: f32,
    pub z_dps: f32,
}

#[derive(defmt::Format)]
pub struct ImuData {
    pub accel: ImuAccel,
    pub gyro: ImuGyro,
}

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

pub struct Mpu6500 {
    spi: Spi<'static, SPI0, Async>,
    cs: Output<'static>,
}

impl Mpu6500 {
    pub fn new(spi: Spi<'static, SPI0, Async>, cs: Output<'static>) -> Self {
        Self { spi, cs }
    }

    pub async fn init(&mut self) {
        self.spi.write(&[CONFIG, 0b0000_0000]).await.unwrap();
        self.spi.write(&[GYRO_CONFIG, 0b0000_1011]).await.unwrap();
        self.spi.write(&[ACCEL_CONFIG, 0b0000_1000]).await.unwrap();
        self.spi.set_frequency(BASE_CLOCK);
    }

    pub async fn read_burst(&mut self) -> Result<RawImuData, SpiError> {
        let mut buf = [0u8; 15];
        buf[0] = ACCEL_GYRO_START;

        // transfer_in_place (of transfer met 1 buffer) stuurt buf en overschrijft het met de gelezen data
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

    pub async fn read_imu(&mut self) -> Result<ImuData, SpiError> {
        let raw = self.read_burst().await?;
        Ok(raw.convert())
    }
}
