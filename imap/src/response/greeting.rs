use util::EncodeTo;

#[derive(Debug, Default)]
pub struct Greeting {}

impl EncodeTo for Greeting {
    fn encode_to(self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(b"* OK IMAP4rev1 Server Ready\r\n");
    }
}
