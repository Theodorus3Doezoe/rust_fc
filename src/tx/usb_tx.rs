// // gonna make this generic in the future
// pub async fn send_msg<D: Driver<'static>, T: Serialize>(
//     sender: &mut Sender<'static, D>,
//     msg: &T,
// ) -> Result<(), ()> {
//     let mut buf = [0u8; 128];
//     let serialized = postcard::to_slice(msg, &mut buf).map_err(|_| ())?;
//     sender.write_packet(serialized).await.map_err(|_| ())
// }
