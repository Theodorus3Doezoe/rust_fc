use super::{Imu, ImuBurst, Rates, Vec3};
use embedded_hal_async::spi::{Operation, SpiDevice};

// Data read start registers & scale factors
const ACCEL_START: u8 = 0x3B | 0x80;
const GYRO_START: u8 = 0x43 | 0x80;
const ACCEL_SCALE: f32 = 8192.0; // ±4g range
const GYRO_SCALE: f32 = 65.5; // ±500 °/s range

const GYRO_TO_RAD: f32 = (1.0 / GYRO_SCALE).to_radians();
const ACCEL_TO_MS2: f32 = 9.81 / ACCEL_SCALE;

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
    async fn read_accel(&mut self) -> Result<Vec3, SPI::Error> {
        let mut buf = [0u8; 7];
        buf[0] = ACCEL_START;

        self.spi.transfer_in_place(&mut buf).await?;

        let raw_x = i16::from_be_bytes([buf[1], buf[2]]) as f32;
        let raw_y = i16::from_be_bytes([buf[3], buf[4]]) as f32;
        let raw_z = i16::from_be_bytes([buf[5], buf[6]]) as f32;

        Ok(Vec3 {
            x: raw_x * ACCEL_TO_MS2,
            y: raw_y * ACCEL_TO_MS2,
            z: raw_z * ACCEL_TO_MS2,
        })
    }

    async fn read_gyro(&mut self) -> Result<Rates, SPI::Error> {
        let mut buf = [0u8; 7];
        buf[0] = GYRO_START;

        self.spi.transfer_in_place(&mut buf).await?;

        let raw_roll = i16::from_be_bytes([buf[1], buf[2]]) as f32;
        let raw_pitch = i16::from_be_bytes([buf[3], buf[4]]) as f32;
        let raw_yaw = i16::from_be_bytes([buf[5], buf[6]]) as f32;

        Ok(Rates {
            roll: raw_roll * GYRO_TO_RAD,
            pitch: raw_pitch * GYRO_TO_RAD,
            yaw: raw_yaw * GYRO_TO_RAD,
        })
    }

    async fn read_burst(&mut self) -> Result<ImuBurst, SPI::Error> {
        let mut buf = [0u8; 15];
        buf[0] = ACCEL_START;

        self.spi.transfer_in_place(&mut buf).await?;

        let raw_ax = i16::from_be_bytes([buf[1], buf[2]]) as f32;
        let raw_ay = i16::from_be_bytes([buf[3], buf[4]]) as f32;
        let raw_az = i16::from_be_bytes([buf[5], buf[6]]) as f32;

        // skip temp

        let raw_roll = i16::from_be_bytes([buf[9], buf[10]]) as f32;
        let raw_pitch = i16::from_be_bytes([buf[11], buf[12]]) as f32;
        let raw_yaw = i16::from_be_bytes([buf[13], buf[14]]) as f32;

        Ok(ImuBurst {
            accel: Vec3 {
                x: raw_ax * ACCEL_TO_MS2,
                y: raw_ay * ACCEL_TO_MS2,
                z: raw_az * ACCEL_TO_MS2,
            },
            gyro: Rates {
                roll: raw_roll * GYRO_TO_RAD,
                pitch: raw_pitch * GYRO_TO_RAD,
                yaw: raw_yaw * GYRO_TO_RAD,
            },
        })
    }
}
