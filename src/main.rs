#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::config::Config;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::spi::{Async, Config as SpiConfig, Spi};

use embassy_rp::dma;
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, SPI0, USB};
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::{Peripherals, bind_interrupts};

const CONFIG: u8 = 0x1A;
const GYRO_CONFIG: u8 = 0x1B;
const ACCEL_CONFIG: u8 = 0x1C;
const ACCEL_GYRO_START: u8 = 0x3B | 0x80;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>;
});

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Config::default());

    let driver = Driver::new(p.USB, UsbIrqs);
    _spawner.spawn(logger_task(driver).unwrap());

    let mut rig = SensorRig::new(
        p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, p.DMA_CH0, p.DMA_CH1, p.PIN_20, p.PIN_21,
    );
    let mut mpu = rig.mpu();

    mpu.init().await;

    loop {
        let raw = mpu.read_burst().await;
        let imu = raw.convert();

        log::info!(
            "accel_x:{} accel_y:{} accel_z:{} gyro_x:{} gyro_y:{} gyro_z:{}",
            imu.accel_x_g,
            imu.accel_y_g,
            imu.accel_z_g,
            imu.gyro_x_dps,
            imu.gyro_y_dps,
            imu.gyro_z_dps
        );

        embassy_time::Timer::after_millis(10).await;
    }
}

struct SensorRig {
    spi: Spi<'static, SPI0, Async>,
    gyro_cs: Output<'static>,
    baro_cs: Output<'static>,
}

impl SensorRig {
    fn new(
        spi0: Peri<'static, SPI0>,
        sck: Peri<'static, embassy_rp::peripherals::PIN_18>,
        mosi: Peri<'static, embassy_rp::peripherals::PIN_19>,
        miso: Peri<'static, embassy_rp::peripherals::PIN_16>,
        dma0: Peri<'static, DMA_CH0>,
        dma1: Peri<'static, DMA_CH1>,
        gyro_pin: Peri<'static, embassy_rp::peripherals::PIN_20>,
        baro_pin: Peri<'static, embassy_rp::peripherals::PIN_21>,
    ) -> Self {
        let mut spi_config = SpiConfig::default();
        spi_config.frequency = 1_000_000;

        let spi = Spi::new(spi0, sck, mosi, miso, dma0, dma1, Irqs, spi_config);
        let gyro_cs = Output::new(gyro_pin, Level::High);
        let baro_cs = Output::new(baro_pin, Level::High);

        Self {
            spi,
            gyro_cs,
            baro_cs,
        }
    }

    fn mpu(&mut self) -> Mpu6500<'_> {
        Mpu6500::new(&mut self.spi, &mut self.gyro_cs)
    }
}

struct Mpu6500<'a> {
    spi: &'a mut Spi<'static, SPI0, Async>,
    cs: &'a mut Output<'static>,
}

#[derive(defmt::Format)]
struct ImuData {
    accel_x_g: f32,
    accel_y_g: f32,
    accel_z_g: f32,
    gyro_x_dps: f32,
    gyro_y_dps: f32,
    gyro_z_dps: f32,
}

impl<'a> Mpu6500<'a> {
    fn new(spi: &'a mut Spi<'static, SPI0, Async>, cs: &'a mut Output<'static>) -> Self {
        Self { spi, cs }
    }

    async fn init(&mut self) {
        self.cs.set_low();
        self.spi.write(&[CONFIG, 0b0000_0011]).await.unwrap();
        self.cs.set_high();

        self.cs.set_low();
        self.spi.write(&[GYRO_CONFIG, 0b0000_1000]).await.unwrap();
        self.cs.set_high();

        self.cs.set_low();
        self.spi.write(&[ACCEL_CONFIG, 0b0000_1000]).await.unwrap();
        self.cs.set_high();
    }

    async fn read_burst(&mut self) -> RawImuData {
        let mut tx_buf = [0u8; 15];
        tx_buf[0] = ACCEL_GYRO_START;
        let mut rx_buf = [0u8; 15];

        self.cs.set_low();
        self.spi.transfer(&mut rx_buf, &tx_buf).await.unwrap();
        self.cs.set_high();

        RawImuData {
            accel_x: i16::from_be_bytes([rx_buf[1], rx_buf[2]]),
            accel_y: i16::from_be_bytes([rx_buf[3], rx_buf[4]]),
            accel_z: i16::from_be_bytes([rx_buf[5], rx_buf[6]]),
            temp: i16::from_be_bytes([rx_buf[7], rx_buf[8]]),
            gyro_x: i16::from_be_bytes([rx_buf[9], rx_buf[10]]),
            gyro_y: i16::from_be_bytes([rx_buf[11], rx_buf[12]]),
            gyro_z: i16::from_be_bytes([rx_buf[13], rx_buf[14]]),
        }
    }
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
    fn convert(self) -> ImuData {
        const ACCEL_SENSITIVITY: f32 = 8192.0;
        const GYRO_SENSITIVITY: f32 = 65.5;
        ImuData {
            accel_x_g: self.accel_x as f32 / ACCEL_SENSITIVITY,
            accel_y_g: self.accel_y as f32 / ACCEL_SENSITIVITY,
            accel_z_g: self.accel_z as f32 / ACCEL_SENSITIVITY,
            gyro_x_dps: self.gyro_x as f32 / GYRO_SENSITIVITY,
            gyro_y_dps: self.gyro_y as f32 / GYRO_SENSITIVITY,
            gyro_z_dps: self.gyro_z as f32 / GYRO_SENSITIVITY,
        }
    }
}

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}
