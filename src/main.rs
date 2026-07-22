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

mod filters;
use filters::filters::GyroFilter;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Config::default());

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
