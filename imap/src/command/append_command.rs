use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct AppendCommand {
    pub tag: String,
    pub mailbox: String,
    pub flags: Vec<String>,
    pub date_time: Option<String>,
    pub message_length: usize,
    pub message: Option<Vec<u8>>,
}

impl ClientCommandTrait for AppendCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let mailbox = cursor.string().unwrap().to_string();

        // TODO: Any error will assume it meant empty flags
        let flags = cursor
            .paren_list(|c| c.flag().map(|s| s.to_string()))
            .unwrap_or_default();

        let date_time = if cursor.peek_nonspace() == Some(b'"') {
            Some(cursor.string()?.to_string())
        } else {
            None
        };

        cursor.eat(b'{')?;
        let message_length = cursor.number()? as usize;
        let sending_now = cursor.eat(b'+').is_ok();
        cursor.eat(b'}')?;
        cursor.eat(b'\r')?;
        cursor.eat(b'\n')?;

        let message = if sending_now {
            let message = cursor.raw(message_length);
            cursor.eat(b'\r')?;
            cursor.eat(b'\n')?;
            Some(message)
        } else {
            None
        };

        Ok(Self {
            tag,
            mailbox,
            flags,
            date_time,
            message_length,
            message,
        })
    }

    fn tag(&self) -> &str {
        &self.tag
    }
}

client_command_from_impl!(AppendCommand, Append);

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(buf: &[u8]) -> (AppendCommand, usize) {
        let (cmd, read) = ClientCommand::parse_bytes(buf).expect("APPEND should parse");
        let ClientCommand::Append(cmd) = cmd else {
            panic!("expected APPEND, got {cmd:?}");
        };
        (cmd, read)
    }

    #[test]
    fn parses_minimal_nonsync_literal() {
        let (cmd, read) = parse(b"a1 APPEND INBOX {13+}\r\nHello, World!\r\n");

        assert_eq!(cmd.tag, "a1");
        assert_eq!(cmd.mailbox, "INBOX");
        assert!(cmd.flags.is_empty());
        assert_eq!(cmd.date_time, None);
        assert_eq!(cmd.message.as_deref(), Some(&b"Hello, World!"[..]));
        // The whole command, literal and trailing CRLF included, is consumed.
        assert_eq!(read, b"a1 APPEND INBOX {13+}\r\nHello, World!\r\n".len());
    }

    #[test]
    fn parses_quoted_mailbox_with_space() {
        let (cmd, _) = parse(b"a1 APPEND \"Sent Items\" {5+}\r\nhello\r\n");

        assert_eq!(cmd.mailbox, "Sent Items");
        assert_eq!(cmd.message.as_deref(), Some(&b"hello"[..]));
    }

    #[test]
    fn parses_flag_list() {
        let (cmd, _) = parse(b"a1 APPEND INBOX (\\Seen \\Draft) {5+}\r\nhello\r\n");

        assert_eq!(cmd.flags, vec!["\\Seen".to_string(), "\\Draft".to_string()]);
        assert_eq!(cmd.date_time, None);
    }

    #[test]
    fn parses_empty_flag_list() {
        let (cmd, _) = parse(b"a1 APPEND INBOX () {5+}\r\nhello\r\n");

        assert!(cmd.flags.is_empty());
    }

    #[test]
    fn parses_date_time() {
        let (cmd, _) = parse(b"a1 APPEND INBOX \"23-Oct-2024 19:00:00 +0000\" {5+}\r\nhello\r\n");

        assert_eq!(cmd.date_time.as_deref(), Some("23-Oct-2024 19:00:00 +0000"));
        assert!(cmd.flags.is_empty());
    }

    #[test]
    fn parses_flags_and_date_time_together() {
        let (cmd, _) =
            parse(b"a1 APPEND INBOX (\\Seen) \"23-Oct-2024 19:00:00 +0000\" {5+}\r\nhello\r\n");

        assert_eq!(cmd.flags, vec!["\\Seen".to_string()]);
        assert_eq!(cmd.date_time.as_deref(), Some("23-Oct-2024 19:00:00 +0000"));
    }

    #[test]
    fn synchronizing_literal_defers_the_message() {
        // No `+` in the literal: the client is waiting for the server's `+`
        // continuation, so the message body is not on the wire yet.
        let (cmd, _) = parse(b"a1 APPEND INBOX {13}\r\n");

        assert_eq!(cmd.mailbox, "INBOX");
        assert_eq!(cmd.message_length, 13);
        assert_eq!(cmd.message, None);
    }

    #[test]
    fn message_bytes_are_preserved_verbatim() {
        let raw = b"a1 APPEND INBOX {26+}\r\nSubject: hi\r\n\r\nbody line\r\n\r\n";
        let (cmd, _) = parse(raw);

        assert_eq!(
            cmd.message.as_deref(),
            Some(&b"Subject: hi\r\n\r\nbody line\r\n"[..]),
        );
        assert_eq!(cmd.message.as_deref().unwrap().len(), 26);
    }

    #[test]
    fn reports_bytes_consumed_for_pipelined_input() {
        // A second command follows the APPEND in the same buffer; only the
        // APPEND's bytes should be reported as consumed.
        let head = b"a1 APPEND INBOX {5+}\r\nhello\r\n";
        let mut raw = head.to_vec();
        raw.extend_from_slice(b"a2 NOOP\r\n");

        let (_, read) = parse(&raw);
        assert_eq!(read, head.len());
    }
}
