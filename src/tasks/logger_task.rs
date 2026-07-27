use crate::config::ActiveUsbDriver;

#[embassy_executor::task]
pub async fn logger_task(driver: ActiveUsbDriver) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}
