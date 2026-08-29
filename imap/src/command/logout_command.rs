use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
};

#[derive(Debug)]
pub struct LogoutCommand {
    pub tag: String,
}

impl ClientCommandTrait for LogoutCommand {
    fn parse_bytes(tag: String, cursor: &mut crate::cursor::Cursor) -> Self {
        cursor.eat(b'\r');
        cursor.eat(b'\n');
        Self { tag }
    }
}

client_command_from_impl!(LogoutCommand, Logout);
