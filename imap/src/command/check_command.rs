use crate::{
    command::client_command::ClientCommandTrait, cursor::Cursor, errors::CommandParseError,
};

#[derive(Debug)]
pub struct CheckCommand {
    pub tag: String,
}

impl ClientCommandTrait for CheckCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        cursor.expect_crlf()?;

        Ok(CheckCommand { tag })
    }
}
