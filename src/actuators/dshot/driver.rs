pub trait EscDriver {
    fn try_send(&mut self, frame: u16) -> bool;
    fn try_recv(&mut self) -> Option<u64>;
}
