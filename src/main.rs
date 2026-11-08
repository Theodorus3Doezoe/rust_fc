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
    sync::{AtomicF32, AtomicRates, ImuQueue, SignaledQueue},
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
static ESC_QUEUE: StaticCell<SignaledQueue<(usize, u64), 16>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut platform = platform::Platform::init().await;
    let queue = IMU_QUEUE.init(Queue::new());
    let (producer, consumer) = queue.split();

    let esc_telem_queue = ESC_QUEUE.init(SignaledQueue::new());
    let (esc_telem_tx, esc_telem_rx) = esc_telem_queue.split();

    platform.rate_spawner.spawn(
        rate_task(
            platform.imu,
            platform.frame,
            producer,
            &RATE_SETPOINTS,
            &THROTTLE,
            esc_telem_tx,
        )
        .unwrap(),
    );

    spawner.spawn(esc_task(esc_telem_rx).unwrap());

    // Receiver task
    spawner.spawn(receiver_task(platform.rx, &THROTTLE, &PILOT_INPUT).unwrap());
    spawner.spawn(attitude_task(consumer, &PILOT_INPUT, &RATE_SETPOINTS).unwrap());

    spawner.spawn(usb_task(platform.usb_dev).unwrap());

    SYSTEM.set_state(State::Disarmed);
}
