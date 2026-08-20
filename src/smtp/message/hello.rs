#[derive(Debug)]
pub struct Hello {
    pub domain: String,
}

impl From<&str> for Hello {
    fn from(value: &str) -> Self {
        dbg!(&value);

        let domain = value.trim_end().to_string();
        Self { domain }
    }
}
