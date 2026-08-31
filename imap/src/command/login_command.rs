use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct LoginCommand {
    pub tag: String,
    pub user: String,
    pub pass: String,
}

impl ClientCommandTrait for LoginCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let user = cursor.atom()?.to_string();
        let pass = cursor.atom()?.to_string();

        Ok(Self { tag, user, pass })
    }
}

client_command_from_impl!(LoginCommand, Login);
