use crate::util::encode_to::EncodeTo;

#[derive(Debug)]
pub struct Ready {
    message: String,
}

impl Ready {
    pub fn new() -> Self {
        Self {
            message: "rs-smtp v0.1".to_string(),
        }
    }
}

impl EncodeTo for Ready {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let message = format!("220 {} ready", self.message);
        buf.extend(message.bytes());
    }
}
