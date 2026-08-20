#[derive(Debug)]
pub struct ExtendedHelloMessage {
    pub domain: String,
}

impl From<&str> for ExtendedHelloMessage {
    fn from(value: &str) -> Self {
        let domain = value.trim_end().to_string();
        Self { domain }
    }
}
