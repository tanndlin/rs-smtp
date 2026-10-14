use crate::{
    command::client_command::ClientCommandTrait, cursor::Cursor, errors::CommandParseError,
};

#[derive(Debug)]
pub struct StartTLSCommand {
    pub tag: String,
}

impl ClientCommandTrait for StartTLSCommand {
    fn parse_bytes(_tag: String, _cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        todo!()
    }
}
