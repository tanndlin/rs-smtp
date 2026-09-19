use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
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

    fn tag(&self) -> &str {
        &self.tag
    }
}

client_command_from_impl!(CheckCommand, Check);
