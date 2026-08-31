use std::fmt;
use std::num::ParseIntError;
use std::panic::Location;

/// A parse failure, carrying enough context to actually debug it:
/// - `kind`: what went wrong
/// - `pos`: byte offset into the command buffer where the parser stopped
/// - `at`: the source location in the parser that raised the error
#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub pos: usize,
    pub at: &'static Location<'static>,
}

#[derive(Debug)]
pub enum ParseErrorKind {
    OutOfBytes,
    ExpectedAtom,
    UnexpectedChar,
    ExpectedNumber,
    ParseIntError(ParseIntError),
}

impl ParseError {
    #[track_caller]
    pub fn new(kind: ParseErrorKind, pos: usize) -> Self {
        Self {
            kind,
            pos,
            at: Location::caller(),
        }
    }

    #[track_caller]
    pub fn out_of_bytes(pos: usize) -> Self {
        Self::new(ParseErrorKind::OutOfBytes, pos)
    }

    #[track_caller]
    pub fn expected_atom(pos: usize) -> Self {
        Self::new(ParseErrorKind::ExpectedAtom, pos)
    }

    #[track_caller]
    pub fn unexpected_char(pos: usize) -> Self {
        Self::new(ParseErrorKind::UnexpectedChar, pos)
    }

    #[track_caller]
    pub fn expected_number(pos: usize) -> Self {
        Self::new(ParseErrorKind::ExpectedNumber, pos)
    }

    /// Render the offending buffer with a caret under the failing byte.
    pub fn render(&self, buf: &[u8]) -> String {
        let line = String::from_utf8_lossy(buf);
        let line = line.replace('\r', "\\r").replace('\n', "\\n");
        // Each escaped \r / \n widened the prefix by one; count them before `pos`.
        let widened = buf[..self.pos.min(buf.len())]
            .iter()
            .filter(|&&b| b == b'\r' || b == b'\n')
            .count();
        let caret_col = self.pos + widened;
        format!(
            "{line}\n{:>col$}\n  {:?} at buffer offset {} (raised at {})",
            "^",
            self.kind,
            self.pos,
            self.at,
            col = caret_col + 1,
        )
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} at offset {} (raised at {})",
            self.kind, self.pos, self.at
        )
    }
}

impl std::error::Error for ParseError {}

impl From<ParseIntError> for ParseError {
    #[track_caller]
    fn from(e: ParseIntError) -> Self {
        Self::new(ParseErrorKind::ParseIntError(e), 0)
    }
}
