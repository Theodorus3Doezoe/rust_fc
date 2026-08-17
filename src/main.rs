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

use crate::{attitude::attitude_task, rate::rate_task, types::ImuBurst};

use heapless::spsc::Queue;
use static_cell::StaticCell;

static IMU_QUEUE: StaticCell<Queue<ImuBurst, 16>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let platform = platform::Platform::init().await;

    let queue = IMU_QUEUE.init(Queue::new());
    let (producer, consumer) = queue.split();

    spawner.spawn(rate_task(platform.imu, producer).unwrap());
    spawner.spawn(attitude_task(consumer).unwrap());
}
