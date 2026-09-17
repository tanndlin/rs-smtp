use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct CreateCommand {
    pub tag: String,
    pub mailbox: String,
}

impl ClientCommandTrait for CreateCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let mailbox = cursor.atom()?.to_string();
        Ok(CreateCommand { tag, mailbox })
    }

    fn tag(&self) -> &str {
        &self.tag
    }
}

client_command_from_impl!(CreateCommand, Create);
