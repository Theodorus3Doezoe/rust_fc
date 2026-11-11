//types for Imu queue
use crate::{
    config::{
        RATE_FREQ_HZ,
        pid::{self, I_LIMIT, PidGains},
    },
    controllers::pid_controller::PidConfig,
    types::{ImuBurst, Rates},
};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use embassy_sync::signal::Signal;
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex},
    channel::Channel,
};
use heapless::spsc::{Consumer, Producer, Queue};

pub static PING_SIGNAL: Signal<CriticalSectionRawMutex, (usize, [u8; 8])> = Signal::new();

// queue for imu data
pub const IMU_QUEUE_CAPACITY: usize = 16;
pub type ImuQueue = Queue<ImuBurst, IMU_QUEUE_CAPACITY>;
pub type ImuProducer = Producer<'static, ImuBurst>;
pub type ImuConsumer = Consumer<'static, ImuBurst>;

// atomic for tuning heartbeat
pub static LAST_SMP: AtomicU32 = AtomicU32::new(0);

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

pub struct AtomicPid {
    kp: AtomicF32,
    ki: AtomicF32,
    kd: AtomicF32,
}

// double with in config.rs and should be in types
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct PidTune {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
}

impl AtomicPid {
    pub const fn new(v: PidGains) -> Self {
        Self {
            kp: AtomicF32::new(v.kp),
            ki: AtomicF32::new(v.ki),
            kd: AtomicF32::new(v.kd),
        }
    }

    pub fn set(&self, v: PidTune) {
        self.kp.set(v.kp);
        self.ki.set(v.ki);
        self.kd.set(v.kd);
    }

    pub fn get(&self) -> PidTune {
        PidTune {
            kp: self.kp.get(),
            ki: self.ki.get(),
            kd: self.kd.get(),
        }
    }

    pub fn pid_configs() -> [PidConfig; 3] {
        let gains = [
            PID_VALS[ROLL_AXIS].get(),
            PID_VALS[PITCH_AXIS].get(),
            PID_VALS[YAW_AXIS].get(),
        ];
        gains.map(|g| PidConfig {
            kp: g.kp,
            ki: g.ki,
            kd: g.kd,
            i_limit: I_LIMIT,
            dterm_cutoff_lpf1_hz: FILTER_CUTOFFS[LPF1_CUTOFF].get(),
            dterm_cutoff_lpf2_hz: FILTER_CUTOFFS[LPF2_CUTOFF].get(),
            dterm_sample_rate: RATE_FREQ_HZ as f32,
        })
    }
}
pub const ROLL_AXIS: usize = 0;
pub const PITCH_AXIS: usize = 1;
pub const YAW_AXIS: usize = 2;

// struct instead of array?
pub static PID_VALS: [AtomicPid; 3] = [
    AtomicPid::new(pid::ROLL),
    AtomicPid::new(pid::PITCH),
    AtomicPid::new(pid::YAW),
];

pub const GYRO_CUTOFF: usize = 0;
pub const LPF1_CUTOFF: usize = 1;
pub const LPF2_CUTOFF: usize = 2;

pub static FILTER_CUTOFFS: [AtomicF32; 3] = [
    AtomicF32::new(80.0),
    AtomicF32::new(150.0),
    AtomicF32::new(250.0),
];

pub static STREAM_ATOMICBOOL: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum TunAck {
    Pid {
        axis: u8,
        kp: f32,
        ki: f32,
        kd: f32,
        ok: bool,
    },
    Filter {
        gyro: f32,
        lpf1: f32,
        lpf2: f32,
        ok: bool,
    },
}

pub static TUNING_CHANNEL: Channel<CriticalSectionRawMutex, TunAck, 4> = Channel::new();
