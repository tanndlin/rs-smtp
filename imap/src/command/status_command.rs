use crate::{
    client_command_from_impl,
    command::client_command::{ClientCommand, ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct StatusCommand {
    pub tag: String,
    pub mailbox: String,

    // Whether these were asked for
    pub messages: bool,
    pub next_uid: bool,
    pub validity_uid: bool,
    pub unseen: bool,
    pub deleted: bool,
    pub size: bool,
}

impl ClientCommandTrait for StatusCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let mailbox = cursor.string()?.to_string();

        let flags = cursor.paren_list(|c| c.atom())?;
        dbg!(&flags);

        let messages = flags.contains(&"MESSAGES");
        let next_uid = flags.contains(&"UIDNEXT");
        let validity_uid = flags.contains(&"UIDVALIDITY");
        let unseen = flags.contains(&"UNSEEN");
        let deleted = flags.contains(&"DELETED");
        let size = flags.contains(&"SIZE");

        cursor.eat(b'\r')?;
        cursor.eat(b'\n')?;
        Ok(Self {
            tag,
            mailbox,
            messages,
            next_uid,
            validity_uid,
            unseen,
            deleted,
            size,
        })
    }

    fn tag(&self) -> &str {
        &self.tag
    }
}

client_command_from_impl!(StatusCommand, Status);

#[cfg(test)]
mod tests {
    use crate::errors::{ParseError, ParseErrorKind};

    use super::*;

    #[test]
    fn no_optionals_throws() {
        let Err(e) = ClientCommand::parse_bytes(b"A042 STATUS blurdybloop\r\n") else {
            panic!()
        };
        assert!(matches!(
            e,
            CommandParseError::ParseError(ParseError {
                kind: ParseErrorKind::UnexpectedChar,
                pos: 23,
                ..
            })
        ));
    }

    #[test]
    fn parses_single_fetch_parameter() {
        let (ClientCommand::Status(cmd), _) =
            ClientCommand::parse_bytes(b"A042 STATUS blurdybloop MESSAGES\r\n").unwrap()
        else {
            panic!()
        };

        assert_eq!(cmd.tag, "A042");
        assert_eq!(cmd.mailbox, "blurdybloop");
        assert!(cmd.messages);
        assert!(!cmd.next_uid);
    }

    #[test]
    fn parses_all_optionals() {
        let (ClientCommand::Status(cmd), _) = ClientCommand::parse_bytes(
            b"A042 STATUS blurdybloop (MESSAGES UIDNEXT UIDVALIDITY UNSEEN DELETED SIZE)\r\n",
        )
        .unwrap() else {
            panic!()
        };

        assert_eq!(cmd.tag, "A042");
        assert_eq!(cmd.mailbox, "blurdybloop");
        assert!(cmd.next_uid);
        assert!(cmd.messages);
    }
}
