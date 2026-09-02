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
mod helpers;
mod mixers;
mod platform;
mod rate;
mod receiver;
mod rx_task;
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
    sync::{AtomicF32, AtomicRates, ImuQueue},
    usb::usb_task,
};

use heapless::spsc::Queue;
use static_cell::StaticCell;

static IMU_QUEUE: StaticCell<ImuQueue> = StaticCell::new();
static PILOT_INPUT: AtomicRates = AtomicRates::new();
static THROTTLE: AtomicF32 = AtomicF32::new(0.0);
static RATE_SETPOINTS: AtomicRates = AtomicRates::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let platform = platform::Platform::init().await;

    let queue = IMU_QUEUE.init(Queue::new());
    let (producer, consumer) = queue.split();
    // Receiver task
    spawner.spawn(rx_task::receiver_task(platform.rx, &THROTTLE, &PILOT_INPUT).unwrap());
    spawner.spawn(attitude_task(consumer, &PILOT_INPUT, &RATE_SETPOINTS).unwrap());
    spawner.spawn(
        rate_task(
            platform.imu,
            platform.frame,
            producer,
            &RATE_SETPOINTS,
            &THROTTLE,
        )
        .unwrap(),
    );

    spawner.spawn(usb_task(platform.usb_dev).unwrap());

    SYSTEM.set_state(State::Disarmed);
}
