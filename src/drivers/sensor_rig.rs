struct SensorRig {
    spi_bus: &'static Spi0Bus,
    gyro_cs: Output<'static>,
    baro_cs: Output<'static>,
}

impl SensorRig {
    fn new(
        spi0: Peri<'static, SPI0>,
        sck: Peri<'static, embassy_rp::peripherals::PIN_18>,
        mosi: Peri<'static, embassy_rp::peripherals::PIN_19>,
        miso: Peri<'static, embassy_rp::peripherals::PIN_16>,
        dma0: Peri<'static, DMA_CH0>,
        dma1: Peri<'static, DMA_CH1>,
        gyro_pin: Peri<'static, embassy_rp::peripherals::PIN_20>,
        baro_pin: Peri<'static, embassy_rp::peripherals::PIN_21>,
    ) -> Self {
        let mut spi_config = SpiConfig::default();
        spi_config.frequency = BASE_CLOCK;

        let spi = Spi::new(spi0, sck, mosi, miso, dma0, dma1, Irqs, spi_config);
        let spi_bus = Mutex::new(spi);
        let spi_bus = SPI_BUS.init(spi_bus);

        let gyro_cs = Output::new(gyro_pin, Level::High);
        let baro_cs = Output::new(baro_pin, Level::High);

        Self {
            spi_bus,
            gyro_cs,
            baro_cs,
        }
    }

    fn mpu(self) -> Mpu6500 {
        Mpu6500::new(self.spi_bus, self.gyro_cs)
    }
}
