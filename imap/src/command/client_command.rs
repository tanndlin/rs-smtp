use imap_derive::ImapCommands;

use crate::{
    command::{
        AppendCommand, CapabilityCommand, CheckCommand, CreateCommand, FetchCommand, ListCommand,
        LoginCommand, LogoutCommand, LsubCommand, NoopCommand, SelectCommand, StartTLSCommand,
        StatusCommand, StoreCommand, UIDCommand,
    },
    cursor::Cursor,
    errors::CommandParseError,
    response::ServerErrorResponse,
};

#[derive(Debug, ImapCommands)]
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
    Store(StoreCommand),
    Check(CheckCommand),
}

impl ClientCommand {
    pub fn parse_bytes(buf: &[u8]) -> Result<(Self, usize), CommandParseError> {
        let mut cursor = Cursor::new(buf);
        let tag = cursor.atom()?.to_string();
        let command_text = cursor.atom()?.to_uppercase();

        let cmd = Self::dispatch(&command_text, tag, &mut cursor)?;

        let bytes_read = cursor.pos;
        Ok((cmd, bytes_read))
    }

    pub fn protocol_violation(&self, reason: String) -> ServerErrorResponse {
        ServerErrorResponse::protocol_violation(self.tag().to_string(), reason)
    }
}

pub trait ClientCommandTrait: Sized {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_tag_does_not_panic() {
        assert!(ClientCommand::parse_bytes(b"a1\r\n").is_err());
    }
}
