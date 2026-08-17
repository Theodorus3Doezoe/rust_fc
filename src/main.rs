#![no_std]
#![no_main]
use defmt_rtt as _;
use panic_probe as _;

mod attitude;
mod boards;
mod config;
mod filters;
mod platform;
mod rate;
mod sensors;
mod types;

use embassy_executor::Spawner;
use embassy_time::Timer;

use crate::{attitude::attitude_task, rate::rate_task};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let platform = platform::Platform::init().await;

    spawner.spawn(rate_task(platform.imu).unwrap());
    spawner.spawn(attitude_task().unwrap());
}
