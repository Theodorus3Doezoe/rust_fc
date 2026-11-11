use crate::boards::UartWrite;
use crate::receiver::crsf::build_frame;
use crate::sync::{PING_SIGNAL, TUNING_CHANNEL};
use embassy_futures::select::{Either, select};

const SUB_PING: u16 = 0x7F03;
const SUB_TUN_ACK: u16 = 0x7F04;
const REPLY_DEST: u8 = 0xEA; // handset / ground station
const REPLY_ORIGIN: u8 = 0xC8; // FC

//TODO should make an build_extended_frame or something else
// must read dest and origin in crsf.rs instead of hardcoded this is temporary
// probally also should make a protocol enum / file or list instead of consts in this file
// so the ground station also can have acces to that if i write it in tauri
// must add the streaming gate / bool
#[embassy_executor::task]
pub async fn tx_telemetry_task(mut tx: crate::config::transmitter::Concrete) {
    let mut n: u32 = 0;
    loop {
        // n = n.wrapping_add(1);
        // if n % 100 == 0 {
        //     defmt::info!("tx alive");
        // }
        defmt::info!("tx alive");
        match select(PING_SIGNAL.wait(), TUNING_CHANNEL.receive()).await {
            Either::First((len, data)) => {
                let mut payload = [0u8; 12];
                payload[0] = REPLY_DEST;
                payload[1] = REPLY_ORIGIN;
                payload[2..4].copy_from_slice(&SUB_PING.to_le_bytes());
                payload[4..4 + len].copy_from_slice(&data[..len]);

                let (buf, n) = build_frame(0xC8, 0x7B, &payload[..4 + len]);
                if tx.write_all(&buf[..n]).await.is_err() {
                    defmt::warn!("tx: ping-echo schrijven mislukt");
                }
            }
            Either::Second(ack) => {
                let mut payload = [0u8; 34];
                payload[0] = REPLY_DEST;
                payload[1] = REPLY_ORIGIN;
                payload[2..4].copy_from_slice(&SUB_TUN_ACK.to_le_bytes());

                let used = match postcard::to_slice(&ack, &mut payload[4..]) {
                    Ok(used) => used.len() + 4,
                    Err(_) => continue,
                };

                let (buf, n) = build_frame(0xC8, 0x7B, &payload[..used]);
                if tx.write_all(&buf[..n]).await.is_err() {
                    defmt::warn!("tx: tun-ack schrijven mislukt");
                }
            }
        }
    }
}
