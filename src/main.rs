#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use embassy_time::{Instant, Timer};

use embassy_executor::{Spawner, task};
use embassy_rp::Peri;
use embassy_rp::config::Config;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::spi::{Async, Config as SpiConfig, Spi};

use embassy_rp::bind_interrupts;
use embassy_rp::dma;
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, SPI0, USB};
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_time::{Duration, Ticker};

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embedded_hal_async::spi::SpiDevice as _;
use static_cell::StaticCell;

type Spi0Bus = Mutex<CriticalSectionRawMutex, Spi<'static, SPI0, Async>>;
static SPI_BUS: StaticCell<Spi0Bus> = StaticCell::new();
struct ImuData {
    accel: ImuAccel,
    gyro: ImuGyro,
}
static IMU_SIGNAL: Signal<CriticalSectionRawMutex, ImuData> = Signal::new();

const CONFIG: u8 = 0x1A;
const GYRO_CONFIG: u8 = 0x1B;
const ACCEL_CONFIG: u8 = 0x1C;
const ACCEL_GYRO_START: u8 = 0x3B | 0x80;
const BASE_CLOCK: u32 = 10_000_000;
const INIT_CLOCK: u32 = 1_000_000;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>;
});

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

mod filters;
use filters::filters::GyroFilter;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Config::default());

    let driver = Driver::new(p.USB, UsbIrqs);
    _spawner.spawn(logger_task(driver).unwrap());

    let rig = SensorRig::new(
        p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, p.DMA_CH0, p.DMA_CH1, p.PIN_20, p.PIN_21,
    );

    let mpu = rig.mpu();

    _spawner.spawn(imu_task(mpu).unwrap());

    let mut ticker = Ticker::every(Duration::from_hz(400));
    let mut latest_imu: Option<ImuData> = None;

    let mut log_counter: u32 = 0;

    loop {
        if let Some(imu) = IMU_SIGNAL.try_take() {
            latest_imu = Some(imu);
        }

        if let Some(imu) = &latest_imu {
            if log_counter.is_multiple_of(40) {
                log::info!(
                    "accel_x:{} accel_y:{} accel_z:{} gyro_x:{} gyro_y:{} gyro_z:{}",
                    imu.accel.x_g,
                    imu.accel.y_g,
                    imu.accel.z_g,
                    imu.gyro.x_dps,
                    imu.gyro.y_dps,
                    imu.gyro.z_dps
                );
            }
            log_counter += 1;
        }

        ticker.next().await;
    }
}

struct SensorRig {
    spi_bus: &'static Spi0Bus,
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
        spi_config.frequency = BASE_CLOCK;

        let spi = Spi::new(spi0, sck, mosi, miso, dma0, dma1, Irqs, spi_config);
        let spi_bus = Mutex::new(spi);
        let spi_bus = SPI_BUS.init(spi_bus);

        let gyro_cs = Output::new(gyro_pin, Level::High);
        let baro_cs = Output::new(baro_pin, Level::High);

        Self {
            spi_bus,
            gyro_cs,
            baro_cs,
        }
    }

    fn mpu(self) -> Mpu6500 {
        Mpu6500::new(self.spi_bus, self.gyro_cs)
    }
}

struct Mpu6500 {
    spi: SpiDevice<'static, CriticalSectionRawMutex, Spi<'static, SPI0, Async>, Output<'static>>,
    spi_bus: &'static Spi0Bus,
}

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

impl Mpu6500 {
    fn new(spi_bus: &'static Spi0Bus, cs: Output<'static>) -> Self {
        Self {
            spi: SpiDevice::new(spi_bus, cs),
            spi_bus,
        }
    }

    async fn init(&mut self) {
        self.spi_bus.lock().await.set_frequency(INIT_CLOCK);
        self.spi.write(&[CONFIG, 0b0000_0000]).await.unwrap(); // DLPF_CFG, genegeerd in bypass
        self.spi.write(&[GYRO_CONFIG, 0b0000_1011]).await.unwrap(); // FS_SEL=1, FCHOICE_B=11 (bypass, ~3600Hz)
        self.spi.write(&[ACCEL_CONFIG, 0b0000_1000]).await.unwrap();
        self.spi_bus.lock().await.set_frequency(BASE_CLOCK);
    }

    async fn read_burst(&mut self) -> Option<RawImuData> {
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

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(4096, log::LevelFilter::Info, driver);
}

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

#[embassy_executor::task]
async fn imu_task(mut mpu: Mpu6500) {
    mpu.init().await;
    defmt::info!("Calibrating, keep sensor still...");
    let bias = calibrate(&mut mpu, 1000).await;
    defmt::info!("Calibrating finished: {}", bias);

    let mut gyro_filter = GyroFilter::new(8000.0, 80.0);
    let mut ticker = Ticker::every(Duration::from_hz(8000));

    let mut timecounter: u32 = 0;
    let mut max_duration: Duration = Duration::from_micros(0);
    let mut time_read_burst: Duration = Duration::from_micros(0);
    const TICK_BUDGET: Duration = Duration::from_hz(8000);

    loop {
        let start = Instant::now();

        let raw = mpu.read_burst().await;
        if let Some(raw) = raw {
            let mut imu = bias.apply(raw.convert());
            time_read_burst = start.elapsed();

            imu.gyro = gyro_filter.apply(imu.gyro);

            IMU_SIGNAL.signal(imu);
        }
        let end = start.elapsed();

        if timecounter.is_multiple_of(400) {
            log::info!(
                "burst_duration:{} total_duration:{} max_duration:{}",
                time_read_burst,
                end,
                max_duration
            );
        }

        if end > TICK_BUDGET {
            defmt::warn!(
                "IMU overrun! Loop duurde {} µs (budget is {} µs)",
                end.as_micros(),
                TICK_BUDGET.as_micros()
            );
        }

        if end > max_duration {
            max_duration = end;
        }
        timecounter += 1;

        ticker.next().await;
    }
}
