use crate::types::Vector3D;

use core::sync::atomic::{AtomicU32, Ordering};

pub struct SetPointRate {
    roll: AtomicU32,
    pitch: AtomicU32,
    yaw: AtomicU32,
}

#[derive(Debug, Clone, Copy, defmt::Format, Default)]
pub struct Axes {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl SetPointRate {
    pub const fn new() -> Self {
        Self {
            roll: AtomicU32::new(0),
            pitch: AtomicU32::new(0),
            yaw: AtomicU32::new(0),
        }
    }

    pub fn set(&self, axes: Axes) {
        self.roll.store(axes.roll.to_bits(), Ordering::Relaxed);
        self.pitch.store(axes.pitch.to_bits(), Ordering::Relaxed);
        self.yaw.store(axes.yaw.to_bits(), Ordering::Relaxed);
    }

    pub fn get(&self) -> Axes {
        Axes {
            roll: f32::from_bits(self.roll.load(Ordering::Relaxed)),
            pitch: f32::from_bits(self.pitch.load(Ordering::Relaxed)),
            yaw: f32::from_bits(self.yaw.load(Ordering::Relaxed)),
        }
    }
}
