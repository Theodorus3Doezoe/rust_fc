pub struct loop_logger {
    counter: u16,
    total_duration_nanos: u64,
    print_hz: u16,
    delay_hz: u16,
}

impl loop_logger {
    pub fn new(print_hz: u16, loop_hz: u16) -> Self {
        Self {
            counter: 0,
            total_duration_nanos: 0,
            print_hz: print_hz,
            delay_hz: loop_hz / print_hz,
        }
    }

    pub fn log() {
        todo!()
    }
}
