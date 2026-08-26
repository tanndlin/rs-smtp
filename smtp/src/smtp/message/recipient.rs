#[derive(Debug)]
pub struct RecipientMessage {
    pub to: String,
}

impl From<&str> for RecipientMessage {
    fn from(value: &str) -> Self {
        let start = value.find("TO:<").expect("Malformed mail command") + "TO:<".len(); // Im pretty sure this len is optimized out
        let end = value[start..].find('>').expect("Malformed mail command") + "TO:<".len(); // Offset added from start

        let to = value[start..end].to_string();
        Self { to }
    }
}
