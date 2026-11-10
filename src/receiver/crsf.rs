use core::sync::atomic::AtomicU32;

use super::receiver::{RcChannels, Receiver};
use crate::state::FlightMode;
use crate::sync::FILTER_CUTOFFS;
use crate::sync::LAST_SMP;
use crate::sync::PING_SIGNAL;
use crate::sync::PidTune;
use crate::sync::STREAM_ATOMICBOOL;
use crate::types::Rates;
use crate::{boards::UartRead, sync::PID_VALS};
use crc::{CRC_8_DVB_S2, Crc};
use crsf::{Packet, PacketParser, RcChannels as RawChannels};
use defmt::println;
use strum::FromRepr;

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

#[derive(Debug, PartialEq, Clone, Copy, FromRepr)]
#[repr(u16)]
pub enum TuningCmd {
    Postcard = 0x7F00,
    StreamOn = 0x7F01,
    StreamOff = 0x7F02,
    Ping = 0x7F03,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub enum TuneMsg {
    SetPid { axis: u8, kp: f32, ki: f32, kd: f32 },
    SetFilter { gyro: f32, lpf1: f32, lpf2: f32 },
}

impl<U: UartRead> Receiver for CrsfReceiver<U> {
    type Error = CrsfError<U::Error>;
    async fn receive(&mut self) -> Result<RcChannels, Self::Error> {
        loop {
            match self.uart.read_exact(&mut self.buf).await {
                Ok(()) => self.parser.push_bytes(&self.buf),
                Err(e) => Err(CrsfError::Uart(e))?,
            }

            //
            while let Some(res) = self.parser.next_raw_packet() {
                match res {
                    Ok(raw) => {
                        match raw.data().get(2) {
                            Some(0x16) => {
                                let ch = Packet::from_raw(&raw);
                                match ch {
                                    Ok(Packet::RcChannels(ch)) => return Ok(map_channels(&ch)),
                                    Err(e) => {
                                        defmt::warn!("crsf: {}", defmt::Debug2Format(&e))
                                    }
                                    _ => {}
                                }
                            }
                            Some(0x7a) => {
                                // Frame: [addr][len][type][dest][orig][func_lo][func_hi][payload..][crc]
                                // body = frame zonder crc.
                                let b = raw.data();
                                let data = b.split_last();
                                if let Some((crc_ref, body)) = data {
                                    let crc: u8 = *crc_ref;

                                    let calculated_crc =
                                        Crc::<u8>::new(&CRC_8_DVB_S2).checksum(&body[2..]);

                                    if crc == calculated_crc {
                                        LAST_SMP.store(
                                            embassy_time::Instant::now().as_millis() as u32,
                                            core::sync::atomic::Ordering::Relaxed,
                                        );
                                        if body.len() >= 7 {
                                            let func = u16::from_le_bytes([body[5], body[6]]);
                                            match TuningCmd::from_repr(func) {
                                                Some(TuningCmd::Postcard) => {
                                                    let Ok(msg) =
                                                        postcard::from_bytes::<TuneMsg>(&body[7..])
                                                    else {
                                                        continue;
                                                    };

                                                    match msg {
                                                        TuneMsg::SetPid { axis, kp, ki, kd } => {
                                                            if (axis as usize) < PID_VALS.len() {
                                                                PID_VALS[axis as usize]
                                                                    .set(PidTune { kp, ki, kd });
                                                            }
                                                        }
                                                        TuneMsg::SetFilter { gyro, lpf1, lpf2 } => {
                                                            for (slot, v) in FILTER_CUTOFFS
                                                                .iter()
                                                                .zip([gyro, lpf1, lpf2])
                                                            {
                                                                slot.set(v);
                                                            }
                                                        }
                                                    }
                                                }
                                                Some(TuningCmd::StreamOn) => {
                                                    STREAM_ATOMICBOOL.store(
                                                        true,
                                                        core::sync::atomic::Ordering::Relaxed,
                                                    );
                                                }
                                                Some(TuningCmd::StreamOff) => {
                                                    STREAM_ATOMICBOOL.store(
                                                        false,
                                                        core::sync::atomic::Ordering::Relaxed,
                                                    );
                                                }
                                                Some(TuningCmd::Ping) => {
                                                    let payload_len = body.len() - 7;
                                                    if payload_len <= 8 {
                                                        let mut buf = [0u8; 8];
                                                        buf[..payload_len]
                                                            .copy_from_slice(&body[7..]);
                                                        PING_SIGNAL.signal((body.len() - 7, buf));
                                                    }
                                                }
                                                None => {}
                                            }
                                        }
                                    } else {
                                        defmt::warn!(
                                            "crsf: Crc does not match in cusom calls: crc: {}, calculated_crc: {}",
                                            crc,
                                            calculated_crc
                                        );
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
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
