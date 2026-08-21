//types for Imu queue
use crate::types::{ImuBurst, Rates};
use core::sync::atomic::{AtomicU32, Ordering};
use heapless::spsc::{Consumer, Producer, Queue};

// queue for imu data
pub const IMU_QUEUE_CAPACITY: usize = 16;
pub type ImuQueue = Queue<ImuBurst, IMU_QUEUE_CAPACITY>;
pub type ImuProducer = Producer<'static, ImuBurst>;
pub type ImuConsumer = Consumer<'static, ImuBurst>;

pub struct AtomicRates {
    roll: AtomicU32,
    pitch: AtomicU32,
    yaw: AtomicU32,
}

impl AtomicRates {
    pub const fn new() -> Self {
        Self {
            roll: AtomicU32::new(0),
            pitch: AtomicU32::new(0),
            yaw: AtomicU32::new(0),
        }
    }

    pub fn set(&self, axes: Rates) {
        self.roll.store(axes.roll.to_bits(), Ordering::Relaxed);
        self.pitch.store(axes.pitch.to_bits(), Ordering::Relaxed);
        self.yaw.store(axes.yaw.to_bits(), Ordering::Relaxed);
    }

    pub fn get(&self) -> Rates {
        Rates {
            roll: f32::from_bits(self.roll.load(Ordering::Relaxed)),
            pitch: f32::from_bits(self.pitch.load(Ordering::Relaxed)),
            yaw: f32::from_bits(self.yaw.load(Ordering::Relaxed)),
        }
    }
}
