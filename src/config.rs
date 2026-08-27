pub use crate::boards::{Board, rp2350dev::Rp2350Dev};
pub use crate::frames::{Frame, v_copter::VCopterFrame};
pub use crate::sensors::{Imu, mpu6500::Mpu6500};

pub type ActiveBoard = Rp2350Dev;
pub type SelectedImuDriver<SPI> = Mpu6500<SPI>;
pub type ActiveFrame<P, M> = VCopterFrame<P, M>;
// pub type ActiveTelemetry = usb::UsbTx;
//

pub mod imu {
    use super::*;
    use crate::sensors::calibrated_imu::CalibratedImu;

    pub const INIT_FREQ_HZ: u32 = 1_000_000;
    pub const RUN_FREQ_HZ: u32 = 20_000_000;

    pub type Spi = <ActiveBoard as Board>::ImuSpi;
    pub type Raw = SelectedImuDriver<Spi>;
    pub type Calibrated = CalibratedImu<Raw>;
}

pub mod frame {

    use crate::boards::ActuatorProvider;

    use super::*;

    pub type ServoPin = <ActiveBoard as ActuatorProvider>::ServoPin;
    pub type MotorPin = <ActiveBoard as ActuatorProvider>::MotorPin;
    pub type Concrete = ActiveFrame<ServoPin, MotorPin>;
}

pub mod receiver {
    pub type Concrete = crate::receiver::usb_receiver::UsbReceiver;
}

// pub mod telemetry {
//     pub type Concrete = crate::usb::UsbTxDriver;
// }

pub const RATE_FREQ_HZ: u16 = 8_000;
pub const GYRO_FILTER_CUTOFF_HZ: f32 = 80.0;

pub const ATTITUDE_FREQ_HZ: u16 = 1_000;

pub const SERVO_FREQ_HZ: u32 = 250;
