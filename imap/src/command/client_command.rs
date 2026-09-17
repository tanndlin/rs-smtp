use crate::{
    command::{
        AppendCommand, CapabilityCommand, CreateCommand, FetchCommand, ListCommand, LoginCommand,
        LogoutCommand, LsubCommand, NoopCommand, SelectCommand, StartTLSCommand, StatusCommand,
        UIDCommand,
    },
    cursor::Cursor,
    errors::CommandParseError,
    response::{ServerErrorReason, ServerErrorResponse},
};

#[derive(Debug)]
pub enum ClientCommand {
    Capability(CapabilityCommand),
    StartTLS(StartTLSCommand),
    Login(LoginCommand),
    List(ListCommand),
    Lsub(LsubCommand),
    Select(SelectCommand),
    Status(StatusCommand),
    Fetch(FetchCommand),
    Append(AppendCommand),
    Logout(LogoutCommand),
    UID(UIDCommand),
    Create(CreateCommand),
    Noop(NoopCommand),
}

impl ClientCommand {
    pub fn parse_bytes(buf: &[u8]) -> Result<(Self, usize), CommandParseError> {
        let mut cursor = Cursor::new(buf);
        let tag = cursor.atom()?.to_string();
        let command_text = cursor.atom()?;

        let cmd = match command_text.to_uppercase().as_str() {
            "CAPABILITY" => CapabilityCommand::parse_bytes(tag, &mut cursor)?.into(),
            "STARTTLS" => StartTLSCommand::parse_bytes(tag, &mut cursor)?.into(),
            "LOGIN" => LoginCommand::parse_bytes(tag, &mut cursor)?.into(),
            "LIST" => ListCommand::parse_bytes(tag, &mut cursor)?.into(),
            "LSUB" => LsubCommand::parse_bytes(tag, &mut cursor)?.into(),
            "SELECT" => SelectCommand::parse_bytes(tag, &mut cursor)?.into(),
            "STATUS" => StatusCommand::parse_bytes(tag, &mut cursor)?.into(),
            "FETCH" => FetchCommand::parse_bytes(tag, &mut cursor)?.into(),
            "APPEND" => AppendCommand::parse_bytes(tag, &mut cursor)?.into(),
            "LOGOUT" => LogoutCommand::parse_bytes(tag, &mut cursor)?.into(),
            "UID" => UIDCommand::parse_bytes(tag, &mut cursor)?.into(),
            "CREATE" => CreateCommand::parse_bytes(tag, &mut cursor)?.into(),
            "NOOP" => NoopCommand::parse_bytes(tag, &mut cursor)?.into(),
            _ => {
                return Err(CommandParseError::MalformedCommand(Some(format!(
                    "unimplemented command {command_text:?}"
                ))));
            }
        };

        let bytes_read = cursor.pos;
        Ok((cmd, bytes_read))
    }
}

pub trait ClientCommandTrait: Sized {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError>;
    fn tag(&self) -> &str;
    fn protocol_violation(self, reason: String) -> ServerErrorResponse {
        ServerErrorResponse {
            tag: Some(self.tag().to_string()),
            reason: ServerErrorReason::ProtocolViolation(reason),
        }
    }
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
