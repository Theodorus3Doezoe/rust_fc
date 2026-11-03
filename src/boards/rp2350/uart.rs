use super::super::{UartRead, UartWrite};
use embassy_rp::uart::{Async, Error, UartRx, UartTx};

impl UartRead for UartRx<'static, Async> {
    type Error = Error;

    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.read(buf).await
    }
}

impl UartWrite for UartTx<'static, Async> {
    type Error = Error;

    async fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.write(buf).await
    }
}
