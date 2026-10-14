use crate::{
    command::client_command::ClientCommandTrait, cursor::Cursor, errors::CommandParseError,
};

#[derive(Debug)]
pub struct StartTLSCommand {
    pub tag: String,
}

impl ClientCommandTrait for StartTLSCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        todo!()
    }
}
