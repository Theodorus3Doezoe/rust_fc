#![no_std]
#![no_main]
use defmt_rtt as _;
use panic_probe as _;

mod actuators;
mod boards;
mod config;
mod controllers;
mod filters;
mod frames;
mod mixers;
mod platform;
mod receiver;
mod sensors;
mod state;
mod sync;
mod tasks;
mod types;

use embassy_executor::Spawner;

use crate::{
    actuators::{
        dshot::state::{claimed_links, link_at},
        provider::ActuatorProvider,
    },
    state::{SYSTEM, State},
    sync::{AtomicF32, AtomicRates, ImuQueue},
    tasks::{
        attitude::attitude_task, esc_telemetry::esc_task, rate::rate_task, rx::receiver_task,
        usb::usb_task,
    },
};

use heapless::spsc::Queue;
use static_cell::StaticCell;

static IMU_QUEUE: StaticCell<ImuQueue> = StaticCell::new();
static PILOT_INPUT: AtomicRates = AtomicRates::new();
static THROTTLE: AtomicF32 = AtomicF32::new(0.0);
static RATE_SETPOINTS: AtomicRates = AtomicRates::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut platform = platform::Platform::init().await;

    for k in 0..claimed_links() {
        let driver = platform.board.take_esc_driver().unwrap();
        let link = link_at(k).unwrap();
        spawner.spawn(esc_task(link, driver).unwrap());
    }

    let queue = IMU_QUEUE.init(Queue::new());
    let (producer, consumer) = queue.split();
    // Receiver task
    spawner.spawn(receiver_task(platform.rx, &THROTTLE, &PILOT_INPUT).unwrap());
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
