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
mod sync;
mod types;
mod usb;

use embassy_executor::Spawner;

use crate::{
    attitude::attitude_task,
    rate::rate_task,
    state::{SYSTEM, State},
    sync::{AtomicRates, ImuQueue},
    usb::{usb_serial_task, usb_task},
};

use heapless::spsc::Queue;
use static_cell::StaticCell;

static IMU_QUEUE: StaticCell<ImuQueue> = StaticCell::new();
static RATE_SETPOINTS: AtomicRates = AtomicRates::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let platform = platform::Platform::init().await;

    let queue = IMU_QUEUE.init(Queue::new());
    let (producer, consumer) = queue.split();

    // spawner.spawn(rate_task(platform.imu, platform.frame, producer, &RATE_SETPOINTS).unwrap());
    spawner.spawn(rate_task(platform.imu, platform.frame, producer, &RATE_SETPOINTS).unwrap());
    spawner.spawn(attitude_task(consumer, &RATE_SETPOINTS).unwrap());

    spawner.spawn(usb_task(platform.usb.dev).unwrap());
    spawner.spawn(usb_serial_task(platform.usb.serial).unwrap());

    SYSTEM.set_state(State::Disarmed);
}
