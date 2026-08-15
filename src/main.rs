#![no_std]
#![no_main]
use defmt_rtt as _;
use panic_probe as _;

mod boards;
mod config;
mod platform;
mod sensors;
mod types;

use embassy_executor::Spawner;
use embassy_time::Timer;

use crate::config::{Imu, imu};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let platform = platform::Platform::init().await;

    spawner.spawn(sensor_task(platform.imu).unwrap());

    loop {
        Timer::after_secs(1).await;
    }
}

use embassy_time::{Duration, Instant, Ticker};

#[embassy_executor::task]
async fn sensor_task(mut imu: imu::Calibrated) {
    let mut ticker = Ticker::every(Duration::from_hz(8000));
    let mut counter: u16 = 0;
    let mut total_duration_nanos: u64 = 0;

    loop {
        ticker.next().await;

        let start = Instant::now();

        let burst = imu.read_burst().await.unwrap();

        total_duration_nanos += start.elapsed().as_nanos();

        counter = (counter + 1) % 16000;

        if counter == 0 {
            let avg_nanos = total_duration_nanos / 16000;
            let avg_micros = avg_nanos / 1000;
            let avg_micros_fraction = (avg_nanos % 1000) / 100;

            defmt::info!(
                "Average read time: {}.{} µs (Budget: 125 µs) | Latest burst: {}",
                avg_micros,
                avg_micros_fraction,
                burst
            );

            total_duration_nanos = 0;
        }
    }
}
