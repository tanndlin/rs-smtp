use util::EncodeTo;

#[derive(Debug)]
pub struct CreateResponse {
    request_tag: String,
    uid_validity: u32,
}
impl CreateResponse {
    pub fn new(tag: String, uid_validity: u32) -> Self {
        CreateResponse {
            request_tag: tag,
            uid_validity,
        }
    }
}

impl EncodeTo for CreateResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self.request_tag.as_bytes());
        buf.extend_from_slice(b" OK [UIDVALIDITY ");
        buf.extend_from_slice(self.uid_validity.to_string().as_bytes());
        buf.extend_from_slice(b"] CREATE completed\r\n");
    }
}
