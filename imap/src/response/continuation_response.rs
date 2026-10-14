use util::EncodeTo;

#[derive(Debug)]
pub struct ContinuationResponse {
    pub tag: String,
}

impl EncodeTo for ContinuationResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(b"+ pls\r\n");
    }
}
