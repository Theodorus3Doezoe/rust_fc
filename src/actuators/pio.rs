use embassy_rp::pio::program::pio_asm;
use embassy_rp::pio::{Pio, PioPin, StateMachine};

pub struct Dshot_pio {
    pin: u8,
}

pub fn pio_dshot() {
    let pio = pio_asm!(
        ".side_set 1",
        ".wrap_target",
        "out pins, 1  side 1",
        "nop          side 0",
        ".wrap",
    );
}
