#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_rp::config::Config;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_rp::init(Config::default());

    loop {
        defmt::info!("Hello from RP2350!");
        embassy_time::Timer::after_secs(1).await;
    }
}
