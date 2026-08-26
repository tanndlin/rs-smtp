pub enum ClientCommand {}

impl From<&[u8]> for ClientCommand {
    fn from(buf: &[u8]) -> Self {
        todo!()
    }
}
