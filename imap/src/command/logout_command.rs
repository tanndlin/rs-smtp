use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct LogoutCommand {
    pub tag: String,
}

impl ClientCommandTrait for LogoutCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        cursor.expect_crlf()?;
        Ok(Self { tag })
    }

    fn tag(&self) -> &str {
        &self.tag
    }
}

client_command_from_impl!(LogoutCommand, Logout);
