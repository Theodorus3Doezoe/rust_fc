pub mod tri_copter;
pub mod v_copter;
use crate::{boards::Board, types::Rates};

pub const MAX_MOTORS: usize = 4;
pub const MAX_SERVOS: usize = 4;

pub trait Frame {
    type Board: Board;
    type Telemetry;
    const MOTOR_COUNT: usize;

    fn init(provider: &mut Self::Board) -> Self;
    fn apply(&mut self, throttle: f32, pid: Rates) -> FrameOutput;
    fn read_telemetry(&mut self) -> heapless::Vec<Option<Self::Telemetry>, MAX_MOTORS>;
    fn stop_all(&mut self);
    // async fn arm_motor(&mut self);
    // fn disarm(&mut self);
    async fn set_direction(&mut self, permanent: bool);
    fn poll_telemetry(&mut self) -> heapless::Vec<Option<u64>, MAX_MOTORS>;
}

#[derive(Default, defmt::Format)]
pub struct FrameOutput {
    pub motors: [f32; MAX_MOTORS],
    pub motor_count: u8,
    pub servo_us: [u16; MAX_SERVOS],
    pub servo_count: u8,
}
