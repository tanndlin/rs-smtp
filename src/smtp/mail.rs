#[derive(Debug)]
pub struct Mail {
    pub from: String,
}

impl From<&str> for Mail {
    fn from(value: &str) -> Self {
        let start = value.find("FROM:<").expect("Malformed mail command") + "FROM:<".len(); // Im pretty sure this len is optimized out
        let end = value[start..].find(">").expect("Malformed mail command") + "FROM:<".len(); // Adding the offset from the start

        let from = value[start..end].to_string();
        Self { from }
    }
}
