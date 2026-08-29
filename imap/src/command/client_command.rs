use crate::{
    command::{
        AppendCommand, CapabilityCommand, FetchCommand, ListCommand, LoginCommand, LogoutCommand,
        SelectCommand, StartTLSCommand, StatusCommand,
    },
    cursor::Cursor,
    response::CommandParseError,
};

#[derive(Debug)]
pub enum ClientCommand {
    Capability(CapabilityCommand),
    StartTLS(StartTLSCommand),
    Login(LoginCommand),
    List(ListCommand),
    Select(SelectCommand),
    Status(StatusCommand),
    Fetch(FetchCommand),
    Append(AppendCommand),
    Logout(LogoutCommand),
}

impl ClientCommand {
    pub fn parse_bytes(buf: &[u8]) -> Result<(Self, usize), CommandParseError> {
        let mut cursor = Cursor::new(buf);
        let tag = cursor
            .atom()
            .map_err(|e| {
                eprintln!("{:?}", e);
                CommandParseError::MalformedCommand
            })?
            .to_string();
        cursor.skip_sp();
        let command_text = cursor.atom().map_err(|e| {
            eprintln!("{:?}", e);
            CommandParseError::MalformedCommand
        })?;

        let cmd = match command_text {
            "CAPABILITY" => CapabilityCommand::parse_bytes(tag, &mut cursor).into(),
            "STARTTLS" => StartTLSCommand::parse_bytes(tag, &mut cursor).into(),
            "LOGIN" => LoginCommand::parse_bytes(tag, &mut cursor).into(),
            "LIST" => ListCommand::parse_bytes(tag, &mut cursor).into(),
            "SELECT" => SelectCommand::parse_bytes(tag, &mut cursor).into(),
            "STATUS" => StatusCommand::parse_bytes(tag, &mut cursor).into(),
            "FETCH" => FetchCommand::parse_bytes(tag, &mut cursor).into(),
            "APPEND" => AppendCommand::parse_bytes(tag, &mut cursor).into(),
            "LOGOUT" => LogoutCommand::parse_bytes(tag, &mut cursor).into(),
            _ => todo!("Probably havent implemented {command_text} yet"),
        };

        let bytes_read = cursor.pos;
        Ok((cmd, bytes_read))
    }
}

pub trait ClientCommandTrait {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Self;
}

#[macro_export]
macro_rules! client_command_from_impl {
    ($type: tt,$variant: ident) => {
        impl From<$type> for ClientCommand {
            fn from(cmd: $type) -> Self {
                ClientCommand::$variant(cmd)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_tag_does_not_panic() {
        assert!(ClientCommand::parse_bytes(b"a1\r\n").is_err());
    }
}
