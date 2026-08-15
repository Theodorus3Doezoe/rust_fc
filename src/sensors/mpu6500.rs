use super::{Imu, ImuBurst, Vector3D};
use embedded_hal_async::spi::{Operation, SpiDevice};

// Data read start registers & scale factors
const ACCEL_START: u8 = 0x3B | 0x80;
const GYRO_START: u8 = 0x43 | 0x80;
const ACCEL_SCALE: f32 = 8192.0; // ±4g range
const GYRO_SCALE: f32 = 65.5; // ±500 °/s range

// Configuration registers
const SMPLRT_DIV: u8 = 0x19;
const CONFIG: u8 = 0x1A;
const GYRO_CONFIG: u8 = 0x1B;
const ACCEL_CONFIG: u8 = 0x1C;
const ACCEL_CONFIG_2: u8 = 0x1D;
const USER_CTRL: u8 = 0x6A;
const PWR_MGMT_1: u8 = 0x6B;

// Configuration values and bitmasks
const GYRO_FS_500: u8 = 0x08;
const ACCEL_FS_4G: u8 = 0x08;
const GYRO_BYPASS: u8 = 0x03; // FCHOICE_B = 11 (bypasses internal gyro DLPF)
const ACCEL_BYPASS: u8 = 0x08; // ACCEL_FCHOICE_B = 1 (bypasses internal accel DLPF)
const PWR_WAKE_AUTO_CLK: u8 = 0x01; // CLKSEL = 1 (auto-selects best available gyro PLL clock)
const I2C_DISABLE: u8 = 0x10; // I2C_IF_DIS = 1 (disables I2C to prevent bus glitches)

pub struct Mpu6500<SPI> {
    spi: SPI,
}

impl<SPI: SpiDevice> Mpu6500<SPI> {
    pub async fn init_registers(spi: &mut SPI) -> Result<(), SPI::Error> {
        let mut rx_buf = [0u8; 1];
        spi.transaction(&mut [
            Operation::Write(&[0x75 | 0x80]),
            Operation::Read(&mut rx_buf),
        ])
        .await?;
        let who_am_i = rx_buf[0];

        assert_eq!(
            who_am_i, 0x70,
            "Error: Wrong chip connected: 0x{:02X}",
            who_am_i
        );

        spi.write(&[PWR_MGMT_1, PWR_WAKE_AUTO_CLK]).await?;
        embassy_time::Timer::after_millis(100).await;
        spi.write(&[USER_CTRL, I2C_DISABLE]).await?;

        spi.write(&[SMPLRT_DIV, 0x00]).await?;
        spi.write(&[CONFIG, 0x00]).await?;
        spi.write(&[GYRO_CONFIG, GYRO_FS_500 | GYRO_BYPASS]).await?;
        spi.write(&[ACCEL_CONFIG, ACCEL_FS_4G]).await?;
        spi.write(&[ACCEL_CONFIG_2, ACCEL_BYPASS]).await?;

        defmt::info!("MPU6500 initalized");
        Ok(())
    }
    pub async fn new(spi: SPI) -> Result<Self, SPI::Error> {
        Ok(Self { spi })
    }
}

impl<SPI: SpiDevice> Imu<SPI> for Mpu6500<SPI> {
    async fn read_accel(&mut self) -> Result<Vector3D, SPI::Error> {
        let mut buf = [0u8; 7];
        buf[0] = ACCEL_START;

        self.spi.transfer_in_place(&mut buf).await?;

        let raw_x = i16::from_be_bytes([buf[1], buf[2]]);
        let raw_y = i16::from_be_bytes([buf[3], buf[4]]);
        let raw_z = i16::from_be_bytes([buf[5], buf[6]]);

        Ok(Vector3D {
            x: raw_x as f32 / ACCEL_SCALE,
            y: raw_y as f32 / ACCEL_SCALE,
            z: raw_z as f32 / ACCEL_SCALE,
        })
    }

    async fn read_gyro(&mut self) -> Result<Vector3D, SPI::Error> {
        let mut buf = [0u8; 7];
        buf[0] = GYRO_START;

        self.spi.transfer_in_place(&mut buf).await?;

        let raw_x = i16::from_be_bytes([buf[1], buf[2]]);
        let raw_y = i16::from_be_bytes([buf[3], buf[4]]);
        let raw_z = i16::from_be_bytes([buf[5], buf[6]]);

        Ok(Vector3D {
            x: raw_x as f32 / GYRO_SCALE,
            y: raw_y as f32 / GYRO_SCALE,
            z: raw_z as f32 / GYRO_SCALE,
        })
    }

    async fn read_burst(&mut self) -> Result<ImuBurst, SPI::Error> {
        let mut buf = [0u8; 15];
        buf[0] = ACCEL_START;

        self.spi.transfer_in_place(&mut buf).await?;

        let raw_ax = i16::from_be_bytes([buf[1], buf[2]]);
        let raw_ay = i16::from_be_bytes([buf[3], buf[4]]);
        let raw_az = i16::from_be_bytes([buf[5], buf[6]]);

        // skip temp

        let raw_gx = i16::from_be_bytes([buf[9], buf[10]]);
        let raw_gy = i16::from_be_bytes([buf[11], buf[12]]);
        let raw_gz = i16::from_be_bytes([buf[13], buf[14]]);

        Ok(ImuBurst {
            accel: Vector3D {
                x: raw_ax as f32 / ACCEL_SCALE,
                y: raw_ay as f32 / ACCEL_SCALE,
                z: raw_az as f32 / ACCEL_SCALE,
            },
            gyro: Vector3D {
                x: raw_gx as f32 / GYRO_SCALE,
                y: raw_gy as f32 / GYRO_SCALE,
                z: raw_gz as f32 / GYRO_SCALE,
            },
        })
    }
}
