use nalgebra::ComplexField;
pub fn create_dshot_frame(throttle: u16, telemetry: bool) -> u16 {
    let throttle = throttle.min(2047);

    let data = (throttle << 1) | (telemetry as u16);

    let crc = (data ^ (data >> 4) ^ (data >> 8)) & 0x0F;

    (data << 4) | crc
}

pub fn mixer_to_dshot_throttle(mixer_value: f32) -> u16 {
    let clamped = mixer_value.clamp(0.0, 1.0);

    let dshot = 48.0 + clamped * (2047.0 - 48.0);
    dshot.round() as u16
}
