use super::receiver::{RcChannels, Receiver};
use crate::boards::UartRead;
use crate::state::FlightMode;
use crate::types::Rates;
use crsf::{Packet, PacketParser, RcChannels as RawChannels};

pub struct CrsfReceiver<U: UartRead> {
    uart: U,
    parser: PacketParser<128>,
    buf: [u8; 64],
}

#[derive(Debug, defmt::Format)]
pub enum CrsfError<E> {
    Uart(E),
}

impl<U: UartRead> CrsfReceiver<U> {
    pub fn new(uart: U) -> Self {
        Self {
            uart,
            parser: PacketParser::new(),
            buf: [0; 64],
        }
    }
}

impl<U: UartRead> Receiver for CrsfReceiver<U> {
    type Error = CrsfError<U::Error>;
    async fn receive(&mut self) -> Result<RcChannels, Self::Error> {
        loop {
            match self.uart.read_exact(&mut self.buf).await {
                Ok(()) => self.parser.push_bytes(&self.buf),
                Err(e) => Err(CrsfError::Uart(e))?,
            }

            while let Some(res) = self.parser.next_packet() {
                match res {
                    Ok((_, Packet::RcChannels(ch))) => {
                        return Ok(map_channels(&ch));
                    }
                    Ok((_, Packet::LinkStatistics(_))) => {}
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }
    }
}

// ch1=roll, ch2=pitch, ch3=thr, ch4=yaw (AETR), ch5=mode(SA),
// ch9=arm(SE)
fn map_channels(raw: &RawChannels) -> RcChannels {
    let roll = norm_stick(raw[0]);
    let pitch = norm_stick(raw[1]);
    let throttle = ((raw[2] as f32 - 172.0) / (1811.0 - 172.0)).clamp(0.0, 1.0);
    let yaw = norm_stick(raw[3]);

    // switches
    let armed = raw[8] > 1500;
    let mode = if raw[4] > 1500 {
        FlightMode::Rate
    } else {
        FlightMode::Angle
    };

    RcChannels {
        rates: Rates { roll, pitch, yaw },
        throttle,
        armed,
        mode,
    }
}

fn norm_stick(v: u16) -> f32 {
    let span = 1811.0 - 992.0;
    ((v as f32 - 992.0) / span).clamp(-1.0, 1.0)
}
