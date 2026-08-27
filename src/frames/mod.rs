pub mod v_copter;

use crate::{boards::Board, frames::v_copter::FrameOutput, types::Rates};

pub trait Frame {
    type Board: Board;
    // type ServoPin;
    // type MotorPin: MotorChannel;
    type Mixer; // could make custom mixer trait 

    fn init(provider: &mut Self::Board) -> Self;
    fn apply(&mut self, throttle: f32, pid: Rates) -> FrameOutput;
    fn stop_all(&mut self);
    async fn arm_motor(&mut self);
    async fn disarm(&mut self);
    async fn set_direction(&mut self, permanent: bool);
}
