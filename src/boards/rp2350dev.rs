use super::Board;

use embassy_rp::bind_interrupts;
use embassy_rp::dma;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, SPI0};
use embassy_rp::spi::{Async, Config as SpiConfig, Spi};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>,
                 dma::InterruptHandler<DMA_CH1>;
});

pub struct Rp2350Dev {
    imu_spi: Option<ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, NoDelay>>,
}

impl Board for Rp2350Dev {
    type ImuSpi = ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, NoDelay>;

    fn init() -> Self {
        let p = embassy_rp::init(Default::default());

        let mut spi_config = SpiConfig::default();
        spi_config.frequency = 1_000_000;

        let imu_spi = Spi::new(
            p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, p.DMA_CH0, p.DMA_CH1, Irqs, spi_config,
        );

        let imu_cs = Output::new(p.PIN_20, Level::High);

        let imu_spi_device = ExclusiveDevice::new_no_delay(imu_spi, imu_cs).unwrap();

        Self {
            imu_spi: Some(imu_spi_device),
        }
    }

    fn take_imu_spi(&mut self) -> Self::ImuSpi {
        self.imu_spi.take().unwrap()
    }
}
