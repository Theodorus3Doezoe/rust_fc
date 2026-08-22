//types for Imu queue
use crate::state::FlightMode;
use crate::types::{ImuBurst, Rates};
use core::sync::atomic::{AtomicU32, Ordering};
use heapless::spsc::{Consumer, Producer, Queue};

// queue for imu data
pub const IMU_QUEUE_CAPACITY: usize = 16;
pub type ImuQueue = Queue<ImuBurst, IMU_QUEUE_CAPACITY>;
pub type ImuProducer = Producer<'static, ImuBurst>;
pub type ImuConsumer = Consumer<'static, ImuBurst>;

pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub const fn new(val: f32) -> Self {
        Self(AtomicU32::new(val.to_bits()))
    }

    pub fn set(&self, val: f32) {
        self.0.store(val.to_bits(), Ordering::Relaxed);
    }

    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
}

pub struct AtomicRates {
    pub roll: AtomicF32,
    pub pitch: AtomicF32,
    pub yaw: AtomicF32,
}

impl AtomicRates {
    pub const fn new() -> Self {
        Self {
            roll: AtomicF32::new(0.0),
            pitch: AtomicF32::new(0.0),
            yaw: AtomicF32::new(0.0),
        }
    }

    pub fn set(&self, axes: Rates) {
        self.roll.set(axes.roll);
        self.pitch.set(axes.pitch);
        self.yaw.set(axes.yaw);
    }

    pub fn get(&self) -> Rates {
        Rates {
            roll: self.roll.get(),
            pitch: self.pitch.get(),
            yaw: self.yaw.get(),
        }
    }
}

// setpoints from sticks
