use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
};

#[derive(Debug)]
pub struct StartTLSCommand {
    pub tag: String,
}

impl ClientCommandTrait for StartTLSCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Self {
        todo!()
    }
}

client_command_from_impl!(StartTLSCommand, StartTLS);
