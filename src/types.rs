#[derive(Debug, Clone, Copy, defmt::Format, Default)]
pub struct Vector3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, defmt::Format, Default)]
pub struct ImuBurst {
    pub accel: Vector3D,
    pub gyro: Vector3D,
}
