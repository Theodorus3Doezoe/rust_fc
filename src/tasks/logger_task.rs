use crate::config::ActiveUsbDriver;
use crate::tasks::logger::{LOG_CHANNEL, LogMessage};
use embassy_usb::Builder;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use static_cell::StaticCell;

#[embassy_executor::task]
pub async fn usb_logger(driver: ActiveUsbDriver) {
    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static STATE: StaticCell<State> = StaticCell::new();

    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
    let msos_descriptor = MSOS_DESCRIPTOR.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 64]);
    let state = STATE.init(State::new());

    let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Ratel Drone");
    config.product = Some("USB Serial");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let mut builder = Builder::new(
        driver,
        config,
        config_descriptor,
        bos_descriptor,
        msos_descriptor,
        control_buf,
    );

    let class = CdcAcmClass::new(&mut builder, state, 64);
    let mut usb = builder.build();

    let (mut sender, _) = class.split();

    let usb_fut = usb.run();
    let send_fut = async {
        loop {
            sender.wait_connection().await;
            loop {
                let msg = LOG_CHANNEL.receive().await;
                let mut buf = [0u8; 32];
                let len = match msg {
                    LogMessage::ImuData {
                        accel_x,
                        accel_y,
                        accel_z,
                        gyro_x,
                        gyro_y,
                        gyro_z,
                    } => {
                        buf[0] = 0xAA;
                        buf[1] = 0xBB;
                        buf[2] = 25; // payload length (1 type + 24 data)
                        buf[3] = 1; // type
                        buf[4..8].copy_from_slice(&accel_x.to_le_bytes());
                        buf[8..12].copy_from_slice(&accel_y.to_le_bytes());
                        buf[12..16].copy_from_slice(&accel_z.to_le_bytes());
                        buf[16..20].copy_from_slice(&gyro_x.to_le_bytes());
                        buf[20..24].copy_from_slice(&gyro_y.to_le_bytes());
                        buf[24..28].copy_from_slice(&gyro_z.to_le_bytes());

                        let mut checksum: u8 = 0;
                        for b in &buf[3..28] {
                            checksum = checksum.wrapping_add(*b);
                        }
                        buf[28] = checksum;
                        29
                    }
                    LogMessage::VqfOrientation { w, x, y, z } => {
                        buf[0] = 0xAA;
                        buf[1] = 0xBB;
                        buf[2] = 17; // payload length (1 type + 16 data)
                        buf[3] = 2; // type
                        buf[4..8].copy_from_slice(&w.to_le_bytes());
                        buf[8..12].copy_from_slice(&x.to_le_bytes());
                        buf[12..16].copy_from_slice(&y.to_le_bytes());
                        buf[16..20].copy_from_slice(&z.to_le_bytes());

                        let mut checksum: u8 = 0;
                        // Checksum berekenen over type (buf[3]) tot en met de laatste data byte (buf[19])
                        for b in &buf[3..20] {
                            checksum = checksum.wrapping_add(*b);
                        }
                        buf[20] = checksum;

                        // Geef de nieuwe totale lengte van het pakket terug (21 bytes)
                        21
                    }
                };

                if let Err(_) = sender.write_packet(&buf[..len]).await {
                    break;
                }
            }
        }
    };

    embassy_futures::join::join(usb_fut, send_fut).await;
}
