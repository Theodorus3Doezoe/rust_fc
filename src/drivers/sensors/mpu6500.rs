const CONFIG: u8 = 0x1A;
const GYRO_CONFIG: u8 = 0x1B;
const ACCEL_CONFIG: u8 = 0x1C;
const ACCEL_GYRO_START: u8 = 0x3B | 0x80;

const BASE_CLOCK: u32 = 10_000_000;
const INIT_CLOCK: u32 = 1_000_000;

#[derive(defmt::Format)]
struct ImuAccel {
    x_g: f32,
    y_g: f32,
    z_g: f32,
}

#[derive(defmt::Format)]
pub struct ImuGyro {
    pub x_dps: f32,
    pub y_dps: f32,
    pub z_dps: f32,
}

pub struct ImuData {
    pub accel: ImuAccel,
    pub gyro: ImuGyro,
}

struct RawImuData {
    accel_x: i16,
    accel_y: i16,
    accel_z: i16,
    temp: i16,
    gyro_x: i16,
    gyro_y: i16,
    gyro_z: i16,
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

struct Mpu6500 {
    spi: SpiDevice<'static, CriticalSectionRawMutex, Spi<'static, SPI0, Async>, Output<'static>>,
    spi_bus: &'static Spi0Bus,
}

impl Mpu6500 {
    pub fn new(spi_bus: &'static Spi0Bus, cs: Output<'static>) -> Self {
        Self {
            spi: SpiDevice::new(spi_bus, cs),
            spi_bus,
        }
    }

    pub async fn init(&mut self) {
        self.spi_bus.lock().await.set_frequency(INIT_CLOCK);
        self.spi.write(&[CONFIG, 0b0000_0000]).await.unwrap(); // DLPF_CFG, genegeerd in bypass
        self.spi.write(&[GYRO_CONFIG, 0b0000_1011]).await.unwrap(); // FS_SEL=1, FCHOICE_B=11 (bypass, ~3600Hz)
        self.spi.write(&[ACCEL_CONFIG, 0b0000_1000]).await.unwrap();
        self.spi_bus.lock().await.set_frequency(BASE_CLOCK);
    }

    pub async fn read_burst(&mut self) -> Option<RawImuData> {
        let mut tx_buf = [0u8; 15];
        tx_buf[0] = ACCEL_GYRO_START;
        let mut rx_buf = [0u8; 15];

        match self.spi.transfer(&mut rx_buf, &tx_buf).await {
            Ok(()) => Some(RawImuData {
                accel_x: i16::from_be_bytes([rx_buf[1], rx_buf[2]]),
                accel_y: i16::from_be_bytes([rx_buf[3], rx_buf[4]]),
                accel_z: i16::from_be_bytes([rx_buf[5], rx_buf[6]]),
                temp: i16::from_be_bytes([rx_buf[7], rx_buf[8]]),
                gyro_x: i16::from_be_bytes([rx_buf[9], rx_buf[10]]),
                gyro_y: i16::from_be_bytes([rx_buf[11], rx_buf[12]]),
                gyro_z: i16::from_be_bytes([rx_buf[13], rx_buf[14]]),
            }),
            Err(e) => {
                defmt::warn!("SPI transfer failed: {}", e);
                None
            }
        }
    }

    // aanpassen voor dit inpv in imu task
    // pub async fn read_imu(&mut self) -> Option<ImuData> {
    //     self.read_burst().await.map(|raw| raw.convert())
    // }
}
