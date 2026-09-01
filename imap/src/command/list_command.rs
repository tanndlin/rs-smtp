use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct ListCommand {
    pub tag: String,
    pub reference_name: String,
    pub mailbox: String,
}

impl ClientCommandTrait for ListCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let reference_name = cursor.string()?.to_string();
        let mailbox = cursor.string()?.to_string();

        cursor.eat(b'\r')?;
        cursor.eat(b'\n')?;

        Ok(Self {
            tag,
            reference_name,
            mailbox,
        })
    }
}

client_command_from_impl!(ListCommand, List);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_flags() {
        let (ClientCommand::List(cmd), _) =
            ClientCommand::parse_bytes(b"A101 LIST \"\" \"\"\r\n").unwrap()
        else {
            panic!()
        };

        assert_eq!(cmd.tag, "A101");
        assert_eq!(cmd.reference_name, "");
        assert_eq!(cmd.mailbox, "");
    }

    #[test]
    fn parses_quoted_reference_name() {
        let (ClientCommand::List(cmd), _) =
            ClientCommand::parse_bytes(b"A101 LIST \"/\" \"\"\r\n").unwrap()
        else {
            panic!()
        };

        assert_eq!(cmd.tag, "A101");
        assert_eq!(cmd.reference_name, "/");
    }

    #[test]
    fn parses_unquoted_reference_name() {
        let (ClientCommand::List(cmd), _) =
            ClientCommand::parse_bytes(b"A101 LIST #news.comp.mail.misc \"\"\r\n").unwrap()
        else {
            panic!()
        };

        assert_eq!(cmd.tag, "A101");
        assert_eq!(cmd.reference_name, "#news.comp.mail.misc");
    }

    #[test]
    fn parses_unquoted_mailbox_name() {
        let (ClientCommand::List(cmd), _) =
            ClientCommand::parse_bytes(b"A101 LIST \"/\" ~/Mail/foo\r\n").unwrap()
        else {
            panic!()
        };

        assert_eq!(cmd.tag, "A101");
        assert_eq!(cmd.reference_name, "/");
        assert_eq!(cmd.mailbox, "~/Mail/foo");
    }
}
