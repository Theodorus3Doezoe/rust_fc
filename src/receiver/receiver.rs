use crate::state::FlightMode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, defmt::Format)]
pub struct RcChannels {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub throttle: f32,
    pub arm: bool,
    pub mode: FlightMode,
}

pub trait Receiver {
    type Error;
    async fn receive(&mut self) -> Result<RcChannels, Self::Error>;
}
