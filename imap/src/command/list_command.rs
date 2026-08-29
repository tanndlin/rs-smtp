use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
};

#[derive(Debug)]
pub struct ListCommand {
    pub tag: String,
}

impl ClientCommandTrait for ListCommand {
    // TODO: Actually parse LIST
    fn parse_bytes(tag: String, cursor: &mut crate::cursor::Cursor) -> Self {
        cursor.eat(b'\r');
        cursor.eat(b'\n');
        Self { tag }
    }
}

client_command_from_impl!(ListCommand, List);
