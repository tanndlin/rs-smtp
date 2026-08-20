#[derive(Debug)]
pub struct ExtendedHello {
    pub domain: String,
}

impl From<&str> for ExtendedHello {
    fn from(value: &str) -> Self {
        dbg!(&value);

        let domain = value.trim_end().to_string();
        Self { domain }
    }
}
