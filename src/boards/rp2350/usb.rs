use super::Irqs;
use embassy_rp::Peri;
use embassy_rp::{peripherals::USB, usb::Driver as RpUsbDriver};

pub fn create_usb(usb: Peri<'static, USB>, irqs: Irqs) -> RpUsbDriver<'static, USB> {
    RpUsbDriver::new(usb, irqs)
}
