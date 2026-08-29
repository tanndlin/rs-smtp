use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
};

#[derive(Debug)]
pub struct LoginCommand {
    pub tag: String,
    pub user: String,
    pub pass: String,
}

impl ClientCommandTrait for LoginCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Self {
        let user = cursor.atom().unwrap().to_string();
        let pass = cursor.atom().unwrap().to_string();

        Self { tag, user, pass }
    }
}

client_command_from_impl!(LoginCommand, Login);
