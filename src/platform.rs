// use crate::config::{Board, BoardType, frame, imu};
use crate::config::{BoardType, frame, receiver};
use crate::usb::UsbDev;
use crate::{
    config::*,
    state::{ArmBlockFlags, SYSTEM, State, SystemState},
    usb::setup_usb,
};
// use crate::config::{Board, BoardType, ConcreteImu, RawImu};

pub struct Platform {
    pub board: BoardType, // can be dropped
    pub imu: imu::Calibrated,
    pub frame: frame::Concrete,
    pub usb_dev: UsbDev,
    pub rx: receiver::Concrete,
}

impl Platform {
    pub async fn init() -> Self {
        let mut board = BoardType::init();
        let mut imu_spi_device = board.take_imu_spi();

        imu::Raw::init_registers(&mut imu_spi_device)
            .await
            .expect("Error: IMU failed to initialize registers");

        imu_spi_device.bus_mut().set_frequency(imu::RUN_FREQ_HZ);

        let raw_imu = imu::Raw::new(imu_spi_device)
            .await
            .expect("Error: IMU failed to initialize");

        let mut imu = imu::Calibrated::new(raw_imu);
        SYSTEM.add_arm_error(ArmBlockFlags::CALIBRATING);
        match imu.calibrate().await {
            Ok(()) => {
                SYSTEM.clear_arm_error(ArmBlockFlags::CALIBRATING);
            }
            Err(_) => {
                defmt::warn!("IMU Calibration failed, keep drone steady");
            }
        }

        let pwm_channels = board.take_pwm_channels(); // todo

        // motor pins

        let frame = frame::Concrete::init(pwm_channels);
        // make the channels more seperated and dynamic
        // add generic servo driver for pwm_channels
        //
        let usb_driver = board.take_usb_driver();

        let usb_dev = crate::usb::UsbManager::init(usb_driver);

        let rx = receiver::Concrete::init();
        // let tx = telemetry::Concrete::init();

        Self {
            board,
            imu,
            frame,
            usb_dev,
            rx,
        }
    }
}
