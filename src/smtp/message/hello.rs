#[derive(Debug)]
pub struct HelloMessage {
    pub domain: String,
}

impl From<&str> for HelloMessage {
    fn from(value: &str) -> Self {
        let domain = value.trim_end().to_string();
        Self { domain }
    }
}
