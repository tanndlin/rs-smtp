use std::fmt;
use std::num::ParseIntError;

use crate::errors::parse_error::ParseError;
use crate::response::ServerErrorResponse;

#[derive(Debug)]
pub enum CommandParseError {
    ParseError(ParseError),
    MalformedCommand(Option<String>),
}

impl CommandParseError {
    /// Human-readable dump of the failure against the raw command bytes,
    /// including a caret at the offending offset and the parser location.
    pub fn render(&self, buf: &[u8]) -> String {
        match self {
            Self::ParseError(e) => e.render(buf),
            Self::MalformedCommand(reason) => format!(
                "{}\nmalformed command: {}",
                String::from_utf8_lossy(buf)
                    .replace('\r', "\\r")
                    .replace('\n', "\\n"),
                reason.as_deref().unwrap_or("<no detail>"),
            ),
        }
    }
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError(e) => write!(f, "{e}"),
            Self::MalformedCommand(reason) => {
                write!(f, "malformed command: {}", reason.as_deref().unwrap_or("?"))
            }
        }
    }
}

impl std::error::Error for CommandParseError {}

impl From<ParseError> for CommandParseError {
    fn from(e: ParseError) -> Self {
        Self::ParseError(e)
    }
}

impl From<ParseIntError> for CommandParseError {
    #[track_caller]
    fn from(e: ParseIntError) -> Self {
        Self::ParseError(e.into())
    }
}

impl From<CommandParseError> for ServerErrorResponse {
    fn from(e: CommandParseError) -> Self {
        Self {
            tag: None,
            reason: crate::response::ServerErrorReason::CommandParseError(e),
        }
    }
}
