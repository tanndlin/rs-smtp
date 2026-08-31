use std::num::ParseIntError;

use crate::errors::parse_error::ParseError;

#[derive(Debug)]
pub enum CommandParseError {
    ParseError(ParseError),
    MalformedCommand(Option<String>),
}

impl From<ParseError> for CommandParseError {
    fn from(e: ParseError) -> Self {
        Self::ParseError(e)
    }
}

impl From<ParseIntError> for CommandParseError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseError(e.into())
    }
}
