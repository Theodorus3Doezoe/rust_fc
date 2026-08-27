use crate::config::{ActiveBoard, Board};
use core::cell::RefCell;
use critical_section::Mutex;
use embassy_usb::class::cdc_acm::{
    CdcAcmClass, Receiver as EmbassyCdcReceiver, Sender as EmbassyCdcSender, State as CdcState,
};
use embassy_usb::driver::Driver;
use embassy_usb::{Builder, Config, UsbDevice};
use static_cell::StaticCell;

pub type ConcreteDriver = <ActiveBoard as Board>::UsbDriver;
pub type UsbRxDriver = EmbassyCdcReceiver<'static, ConcreteDriver>;
pub type UsbTxDriver = EmbassyCdcSender<'static, ConcreteDriver>;
pub type UsbDev = UsbDevice<'static, ConcreteDriver>;

static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static CDC_STATE: StaticCell<CdcState> = StaticCell::new();

pub struct UsbManager {
    rx: Option<UsbRxDriver>,
    tx: Option<UsbTxDriver>,
}

impl UsbManager {
    const fn empty() -> Self {
        Self { rx: None, tx: None }
    }
}

unsafe impl Send for UsbManager {}
pub static USB_MANAGER: Mutex<RefCell<UsbManager>> = Mutex::new(RefCell::new(UsbManager::empty()));

impl UsbManager {
    pub fn init(driver: ConcreteDriver) -> UsbDev {
        let (dev, class) = setup_usb(driver);
        let (tx, rx) = class.split();

        critical_section::with(|cs| {
            let mut mgr = USB_MANAGER.borrow(cs).borrow_mut();
            mgr.rx = Some(rx);
            mgr.tx = Some(tx);
        });

        dev
    }

    pub fn take_rx() -> UsbRxDriver {
        critical_section::with(|cs| {
            USB_MANAGER
                .borrow(cs)
                .borrow_mut()
                .rx
                .take()
                .expect("USB RX already taken or not initialized")
        })
    }

    pub fn take_tx() -> UsbTxDriver {
        critical_section::with(|cs| {
            USB_MANAGER
                .borrow(cs)
                .borrow_mut()
                .tx
                .take()
                .expect("USB TX already taken or not initialized")
        })
    }
}

pub fn setup_usb<D: Driver<'static>>(
    driver: D,
) -> (UsbDevice<'static, D>, CdcAcmClass<'static, D>) {
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("RatelDynamics");
    config.product = Some("FlightController");
    config.serial_number = Some("001");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_DESC.init([0; 256]),
        BOS_DESC.init([0; 256]),
        &mut [],
        CONTROL_BUF.init([0; 64]),
    );

    let class = CdcAcmClass::new(&mut builder, CDC_STATE.init(CdcState::new()), 64);
    let usb_dev = builder.build();

    (usb_dev, class)
}

#[embassy_executor::task]
pub async fn usb_task(mut usb_dev: UsbDev) {
    usb_dev.run().await;
}
