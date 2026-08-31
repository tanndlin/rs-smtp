use std::num::ParseIntError;

#[derive(Debug)]
pub enum ParseError {
    OutOfBytes,
    ExpectedAtom(usize),
    UnexpectedChar(usize),
    ExpectedNumber(usize),
    ParseIntError(ParseIntError),
}

impl From<ParseIntError> for ParseError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseIntError(e)
    }
}
