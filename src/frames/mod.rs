pub mod v_copter;

use crate::{
    frames::v_copter::FrameOutput,
    // mixers::bi_copter::BicopterOutput,
    types::{PwmChannels, Rates},
};
use embedded_hal::pwm::SetDutyCycle;

pub trait Frame {
    type PwmPin: SetDutyCycle;
    type Mixer;

    fn init(channels: PwmChannels<Self::PwmPin>) -> Self;
    fn apply(&mut self, throttle: f32, pid: Rates) -> FrameOutput;
    fn stop_all(&mut self);
}
