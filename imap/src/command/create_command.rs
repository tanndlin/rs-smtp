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
        let mailbox = cursor.string()?.to_string();

        cursor.eat(b'\r')?;
        cursor.eat(b'\n')?;

        Ok(CreateCommand { tag, mailbox })
    }

    fn tag(&self) -> &str {
        &self.tag
    }
}

client_command_from_impl!(CreateCommand, Create);
