const SPI_CLOCK: u32 = 10_000_000;
const SPI_INIT_CLOCK: u32 = 1_000_000;

type Spi0Bus = Mutex<CriticalSectionRawMutex, Spi<'static, SPI0, Async>>;
static SPI_BUS: StaticCell<Spi0Bus> = StaticCell::new();

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>;
});

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

pub struct BoardHardware {
    pub spi_bus: &'static Spi0Bus,
    pub gyro_cs: Output<'static>,
    pub baro_cs: Output<'static>,
    pub usb_driver: Driver<'static, USB>,
}

pub fn init() -> BoardHardware {
    let p = embassy_rp::init(Default::default());

    let usb_driver = Driver::new(p.USB, UsbIrqs);

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = SPI_INIT_CLOCK;

    let spi = Spi::new(
        p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, p.DMA_CH0, p.DMA_CH1, Irqs, spi_config,
    );

    let spi_bus = SPI_BUS.init(Mutex::new(spi));

    let gyro_cs = Output::new(p.PIN_20, Level::High);
    let baro_cs = Output::new(p.PIN_21, Level::High);

    BoardHardware {
        spi_bus,
        gyro_cs,
        baro_cs,
        usb_driver,
    }
}
