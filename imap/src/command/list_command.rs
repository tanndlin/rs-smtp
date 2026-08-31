use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct ListCommand {
    pub tag: String,
}

impl ClientCommandTrait for ListCommand {
    // TODO: Actually parse LIST
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        cursor.eat(b'\r')?;
        cursor.eat(b'\n')?;
        Ok(Self { tag })
    }
}

client_command_from_impl!(ListCommand, List);
