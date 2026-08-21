#![no_std]
#![no_main]
use defmt_rtt as _;
use panic_probe as _;

mod actuators;
mod attitude;
mod boards;
mod config;
mod controllers;
mod filters;
mod frames;
mod mixers;
mod platform;
mod rate;
mod sensors;
mod state;
mod types;

use embassy_executor::Spawner;

use crate::{attitude::attitude_task, rate::rate_task, state::SetPointRate, types::ImuBurst};

use heapless::spsc::Queue;
use static_cell::StaticCell;

static IMU_QUEUE: StaticCell<Queue<ImuBurst, 16>> = StaticCell::new();
static RATE_SETPOINTS: SetPointRate = SetPointRate::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let platform = platform::Platform::init().await;

    let queue = IMU_QUEUE.init(Queue::new());
    let (producer, consumer) = queue.split();

    spawner.spawn(rate_task(platform.imu, platform.frame, producer, &RATE_SETPOINTS).unwrap());
    spawner.spawn(attitude_task(consumer, &RATE_SETPOINTS).unwrap());
}
