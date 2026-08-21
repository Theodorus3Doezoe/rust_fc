use crate::state::{SYSTEM, State};
use crate::sync::AtomicRates;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_usb::class::cdc_acm::{CdcAcmClass, Receiver, Sender, State as CdcState};
use embassy_usb::{Builder, Config, UsbDevice};
use embassy_usb_driver::Driver;
use serde::{Deserialize, Serialize};
use static_cell::StaticCell;

static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static CDC_STATE: StaticCell<CdcState> = StaticCell::new();
use crate::config::usb::{Device as UsbDev, SerialClass};

pub static TELEMETRY_CHANNEL: Channel<CriticalSectionRawMutex, ToPc, 8> = Channel::new();

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ToPc {
    Attitude {
        roll: f32,
        pitch: f32,
        yaw: f32,
    },
    SystemState {
        state: u8,
        arm_blocks: u32,
        errors: u32,
    },
    Ack,
    Log(heapless::String<32>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FromPc {
    Arm,
    Disarm,
}

pub fn setup_usb<D: Driver<'static>>(
    driver: D,
) -> (UsbDevice<'static, D>, CdcAcmClass<'static, D>) {
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("MyDrone");
    config.product = Some("FlightController");
    config.serial_number = Some("001");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let builder = Builder::new(
        driver,
        config,
        CONFIG_DESC.init([0; 256]),
        BOS_DESC.init([0; 256]),
        &mut [],
        CONTROL_BUF.init([0; 64]),
    );

    let mut builder = builder;
    let class = CdcAcmClass::new(&mut builder, CDC_STATE.init(CdcState::new()), 64);
    let usb_dev = builder.build();

    (usb_dev, class)
}

pub async fn send_msg<D: Driver<'static>, T: Serialize>(
    sender: &mut Sender<'static, D>,
    msg: &T,
) -> Result<(), ()> {
    let mut buf = [0u8; 128];
    let serialized = postcard::to_slice(msg, &mut buf).map_err(|_| ())?;
    sender.write_packet(serialized).await.map_err(|_| ())
}

pub async fn receive_msg<D: Driver<'static>, T: for<'de> Deserialize<'de>>(
    receiver: &mut Receiver<'static, D>,
) -> Result<T, ()> {
    let mut rx_buf = [0u8; 64];
    let n = receiver.read_packet(&mut rx_buf).await.map_err(|_| ())?;
    postcard::from_bytes(&rx_buf[..n]).map_err(|_| ())
}

pub async fn run_usb<D: Driver<'static>>(mut usb_dev: UsbDevice<'static, D>) {
    usb_dev.run().await;
}

pub async fn run_serial<D: Driver<'static>>(class: CdcAcmClass<'static, D>) {
    let (mut sender, mut receiver) = class.split();

    loop {
        receiver.wait_connection().await;

        loop {
            let rx_fut = receive_msg::<D, FromPc>(&mut receiver);
            let tx_fut = TELEMETRY_CHANNEL.receive();

            match select(rx_fut, tx_fut).await {
                Either::First(Ok(cmd)) => match cmd {
                    FromPc::Arm => {
                        if SYSTEM.can_arm() {
                            SYSTEM.set_state(State::Armed);
                            let _ = send_msg(&mut sender, &ToPc::Ack).await;
                        }
                    }
                    FromPc::Disarm => {
                        SYSTEM.set_state(State::Disarmed);
                        let _ = send_msg(&mut sender, &ToPc::Ack).await;
                    }
                },
                Either::First(Err(_)) => break,

                Either::Second(msg) => {
                    if send_msg(&mut sender, &msg).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

pub fn publish_telemetry(msg: ToPc) {
    let _ = TELEMETRY_CHANNEL.try_send(msg);
}

#[embassy_executor::task]
pub async fn usb_task(usb_dev: UsbDev) {
    crate::usb::run_usb(usb_dev).await;
}

#[embassy_executor::task]
pub async fn usb_serial_task(class: SerialClass) {
    crate::usb::run_serial(class).await;
}
