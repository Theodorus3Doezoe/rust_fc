//types for Imu queue
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

// can have updates while reading could use seqlock
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

use core::future::poll_fn;
use core::task::Poll;
use embassy_sync::waitqueue::AtomicWaker;
// setpoints from sticks

pub struct SignaledQueue<T, const N: usize> {
    queue: Queue<T, N>,
    waker: AtomicWaker,
}

impl<T, const N: usize> SignaledQueue<T, N> {
    pub const fn new() -> Self {
        Self {
            queue: Queue::new(),
            waker: AtomicWaker::new(),
        }
    }

    pub fn split(&mut self) -> (Sender<'_, T>, Receiver<'_, T>) {
        let (producer, consumer) = self.queue.split();
        (
            Sender {
                producer,
                waker: &self.waker,
            },
            Receiver {
                consumer,
                waker: &self.waker,
            },
        )
    }
}
pub struct Sender<'a, T> {
    producer: Producer<'a, T>,
    waker: &'a AtomicWaker,
}

impl<'a, T> Sender<'a, T> {
    pub fn try_send(&mut self, item: T) -> Result<(), T> {
        let res = self.producer.enqueue(item);
        if res.is_ok() {
            self.waker.wake();
        }
        res
    }
}

pub struct Receiver<'a, T> {
    consumer: Consumer<'a, T>,
    waker: &'a AtomicWaker,
}

impl<'a, T> Receiver<'a, T> {
    pub async fn receive(&mut self) -> T {
        poll_fn(|cx| {
            self.waker.register(cx.waker());

            // 2. Probeer te lezen
            if let Some(item) = self.consumer.dequeue() {
                Poll::Ready(item)
            } else {
                Poll::Pending // 3. Ga slapen
            }
        })
        .await
    }
}
