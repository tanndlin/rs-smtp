use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct LsubCommand {
    pub tag: String,
    pub reference_name: String,
    pub mailbox: String,
}

impl ClientCommandTrait for LsubCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let reference_name = cursor.string()?.to_string();
        let mailbox = cursor.list_mailbox()?.to_string();

        cursor.expect_crlf()?;

        Ok(Self {
            tag,
            reference_name,
            mailbox,
        })
    }

    fn tag(&self) -> &str {
        &self.tag
    }
}

client_command_from_impl!(LsubCommand, Lsub);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_flags() {
        let (ClientCommand::Lsub(cmd), _) =
            ClientCommand::parse_bytes(b"A101 LSUB \"\" \"\"\r\n").unwrap()
        else {
            panic!()
        };

        assert_eq!(cmd.tag, "A101");
        assert_eq!(cmd.reference_name, "");
        assert_eq!(cmd.mailbox, "");
    }

    #[test]
    fn parses_mailbox_wildcard() {
        let (ClientCommand::Lsub(cmd), _) =
            ClientCommand::parse_bytes(b"A101 LSUB \"\" \"*\"\r\n").unwrap()
        else {
            panic!()
        };

        assert_eq!(cmd.tag, "A101");
        assert_eq!(cmd.reference_name, "");
        assert_eq!(cmd.mailbox, "*");
    }
}
