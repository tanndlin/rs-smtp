use crate::Email;

#[derive(Debug)]
pub struct BodyStructure {
    pub content_type: String,
    pub content_subtype: String,
    pub body_parameters: Vec<(String, String)>,
    pub content_id: Option<String>,
    pub content_description: Option<String>,
    pub content_transfer_encoding: Option<String>,
    pub size: usize,
    pub lines: usize,
}

impl From<&Email> for BodyStructure {
    fn from(email: &Email) -> Self {
        let content_type_header = email.header("Content-Type").unwrap_or("text/plain");
        let content_type = content_type_header
            .split_once(';')
            .map(|(t, _)| t)
            .unwrap_or(content_type_header)
            .to_owned();
        let content_subtype = content_type_header
            .split_once('/')
            .map(|(_, s)| s)
            .unwrap_or("plain")
            .to_owned();
        let body_parameters = vec![("CHARSET".to_string(), "US-ASCII".to_string())]; // TODO
        let content_id = email.header("Content-ID").map(str::to_string);
        let content_description = email.header("Content-Description").map(str::to_string);
        let content_transfer_encoding = email
            .header("Content-Transfer-Encoding")
            .map(str::to_string);
        let size = email.body_text.as_ref().map(|b| b.len()).unwrap_or(0);
        let lines = email
            .body_text
            .as_ref()
            .map(|b| b.lines().count())
            .unwrap_or(0);

        Self {
            content_type,
            content_subtype,
            body_parameters,
            content_id,
            content_description,
            content_transfer_encoding,
            size,
            lines,
        }
    }
}
