use embassy_rp::Peri;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, PIN_16, PIN_18, PIN_19, PIN_20, SPI0};
use embassy_rp::spi::{Async, Config as SpiConfig, Spi};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};

pub type ImuConcrete = ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, NoDelay>;

// should be a type not the exact struct?
use crate::boards::rp2350::Irqs;

pub fn create_imu_spi(
    spi: Peri<'static, SPI0>,
    clk: Peri<'static, PIN_18>,
    mosi: Peri<'static, PIN_19>,
    miso: Peri<'static, PIN_16>,
    tx_dma: Peri<'static, DMA_CH0>,
    rx_dma: Peri<'static, DMA_CH1>,
    cs: Peri<'static, PIN_20>,
    irqs: Irqs,
) -> ImuConcrete {
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 1_000_000;

    let imu_spi = Spi::new(spi, clk, mosi, miso, tx_dma, rx_dma, irqs, spi_config);

    let imu_cs = Output::new(cs, Level::High);

    ExclusiveDevice::new_no_delay(imu_spi, imu_cs).unwrap()
}
