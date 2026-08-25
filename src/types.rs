#[derive(Debug, Clone, Copy, defmt::Format, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
#[derive(Debug, Deserialize, Serialize, Clone, Copy, defmt::Format, Default)]
pub struct Rates {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}
#[derive(Debug, Clone, Copy, defmt::Format, Default)]
pub struct ImuBurst {
    pub accel: Vec3,
    pub gyro: Rates,
}

pub struct PwmChannels<P> {
    pub pwm1: P,
    pub pwm2: P,
    pub pwm3: P,
    pub pwm4: P,
}

#[derive(Clone, Copy, defmt::Format)]
pub struct ActuatorOutput {
    pub servo_left_us: u16,
    pub servo_right_us: u16,
    pub motor_left: f32,
    pub motor_right: f32,
}

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ToUsb {
    Attitude {
        roll: f32,
        pitch: f32,
        yaw: f32,
    },
    SystemState {
        state: u8,
        arm_blocks: u32,
        errors: u32,
    },
    Ack,
    Log(heapless::String<32>),
}
