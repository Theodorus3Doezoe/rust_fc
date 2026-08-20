pub mod v_copter;

use crate::types::{PwmChannels, Rates};
use embedded_hal::pwm::SetDutyCycle;

pub trait Frame {
    type PwmPin: SetDutyCycle;
    type Mixer;

    fn init(mixer: Self::Mixer, channels: PwmChannels<Self::PwmPin>) -> Self;
    fn apply(&mut self, throttle: f32, pid: Rates, is_armed: bool);
}
