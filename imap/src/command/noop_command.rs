use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct NoopCommand {
    pub tag: String,
}

impl ClientCommandTrait for NoopCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        cursor.expect_crlf()?;

        Ok(NoopCommand { tag })
    }

    fn tag(&self) -> &str {
        &self.tag
    }
}

client_command_from_impl!(NoopCommand, Noop);
