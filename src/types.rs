#[derive(Debug, Clone, Copy, defmt::Format, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
#[derive(Debug, Clone, Copy, defmt::Format, Default)]
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
    pub motor_left: f32, // of u16 als je DShot waarden hebt
    pub motor_right: f32,
}
