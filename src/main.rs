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

use crate::sensors::Imu;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let system = platform::create_system().await;

    spawner.spawn(sensor_task(system.imu).unwrap());

    loop {
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::task]
async fn sensor_task(mut imu: config::ConcreteImu) {
    let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_hz(8000));
    let mut counter: u16 = 0;
    loop {
        ticker.next().await;
        let burst = imu.read_burst().await.unwrap();

        if counter == 0 {
            defmt::info!("Burst: {}", burst);
        }
        counter = (counter + 1) % 800;
    }
}
