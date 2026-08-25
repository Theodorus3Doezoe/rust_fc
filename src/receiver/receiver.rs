use crate::{state::FlightMode, types::Rates};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, defmt::Format)]
pub struct RcChannels {
    pub rates: Rates,
    pub throttle: f32,
    pub arm: bool,
    pub disarm: bool,
    pub mode: FlightMode,
}

pub trait Receiver {
    type Error;
    async fn receive(&mut self) -> Result<RcChannels, Self::Error>;
}
