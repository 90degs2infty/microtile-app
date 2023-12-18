#[must_use]
pub enum Message {
    TimerTick,
    BtnBPress,
    AccelerometerData { x: i16, z: i16 },
}

impl Message {
    pub fn acceleration(x: i16, z: i16) -> Self {
        Self::AccelerometerData { x, z }
    }
}
