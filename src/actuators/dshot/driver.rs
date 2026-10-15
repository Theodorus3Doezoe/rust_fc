pub trait EscDriver {
    async fn send_frame(&mut self, frame: u16);
    async fn read_pair(&mut self) -> (u32, u32);
}
