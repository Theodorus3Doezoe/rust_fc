use super::receiver::{RcChannels, Receiver};
use crate::usb::{UsbManager, UsbRxDriver};
use embassy_usb::driver::EndpointError;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum UsbReceiverError {
    Usb(EndpointError),
    Decode(postcard::Error),
}

impl From<EndpointError> for UsbReceiverError {
    fn from(e: EndpointError) -> Self {
        UsbReceiverError::Usb(e)
    }
}

impl From<postcard::Error> for UsbReceiverError {
    fn from(e: postcard::Error) -> Self {
        UsbReceiverError::Decode(e)
    }
}

pub struct UsbReceiver {
    rx: UsbRxDriver,
    buffer: [u8; 64],
}

impl UsbReceiver {
    pub fn init() -> Self {
        let rx = UsbManager::take_rx();
        Self {
            rx,
            buffer: [0u8; 64],
        }
    }
}

impl Receiver for UsbReceiver {
    type Error = UsbReceiverError;

    async fn receive(&mut self) -> Result<RcChannels, Self::Error> {
        let n = self.rx.read_packet(&mut self.buffer).await?;
        if n == 0 {};
        let msg: RcChannels = postcard::from_bytes(&self.buffer[..n])?;
        Ok(msg)
    }
}
