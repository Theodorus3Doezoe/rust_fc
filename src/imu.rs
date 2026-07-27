#[derive(defmt::Format, Debug, Clone, Copy)]
pub struct ImuAccel {
    pub x_g: f32,
    pub y_g: f32,
    pub z_g: f32,
}

#[derive(defmt::Format, Debug, Clone, Copy)]
pub struct ImuGyro {
    pub x_dps: f32,
    pub y_dps: f32,
    pub z_dps: f32,
}

#[derive(defmt::Format, Debug, Clone, Copy)]
pub struct ImuData {
    pub accel: ImuAccel,
    pub gyro: ImuGyro,
}

pub trait AdjustableSpiSpeed {
    fn set_frequency(&mut self, freq_hz: u32);
}

pub trait Imu {
    type Error;
    type SpiBus;

    async fn init(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<ImuData, Self::Error>;

    fn spi_device_mut(&mut self) -> &mut Self::SpiBus;
}
