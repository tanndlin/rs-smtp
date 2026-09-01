use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct SelectCommand {
    pub tag: String,
    pub mailbox: String,
}

impl ClientCommandTrait for SelectCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let mailbox = cursor.string().unwrap().to_string();

        cursor.eat(b'\r')?;
        cursor.eat(b'\n')?;

        Ok(Self { tag, mailbox })
    }
}

client_command_from_impl!(SelectCommand, Select);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mailbox() {
        let (ClientCommand::Select(cmd), _) =
            ClientCommand::parse_bytes(b"a69 SELECT INBOX\r\n").unwrap()
        else {
            panic!()
        };

        assert_eq!(cmd.tag, "a69");
        assert_eq!(cmd.mailbox, "INBOX");
    }
}
