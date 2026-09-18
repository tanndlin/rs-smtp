use crate::{
    client_command_from_impl,
    command::{ClientCommand, Sequence, Store, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct StoreCommand {
    pub tag: String,
    pub sequences: Vec<Sequence>,
    pub store: Store,
}

impl ClientCommandTrait for StoreCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let sequences = Sequence::parse_set(cursor)?;
        let store = Store::parse(cursor)?;

        cursor.expect_crlf()?;

        Ok(StoreCommand {
            tag,
            sequences,
            store,
        })
    }

    fn tag(&self) -> &str {
        &self.tag
    }
}

client_command_from_impl!(StoreCommand, Store);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{FetchIndicator, StoreOperation};

    fn parse_store(buf: &[u8]) -> StoreCommand {
        let (cmd, _) = ClientCommand::parse_bytes(buf).unwrap();
        let ClientCommand::Store(cmd) = cmd else {
            panic!("expected ClientCommand::Store, got {cmd:?}");
        };

        cmd
    }

    #[test]
    fn parses_tag_sequences_and_store() {
        let cmd = parse_store(b"a1 STORE 2:4 +FLAGS (\\Deleted)\r\n");

        assert_eq!(cmd.tag, "a1");
        assert!(matches!(
            cmd.sequences[..],
            [Sequence::Range {
                start: FetchIndicator::Index(2),
                end: FetchIndicator::Index(4)
            }]
        ));
        assert_eq!(cmd.store.operation, StoreOperation::Add);
        assert!(!cmd.store.silent);
        assert_eq!(cmd.store.flags, vec!["\\Deleted".to_string()]);
    }

    /// `STORE` is lowercase-tolerant like every other command name.
    #[test]
    fn parses_lowercase_command() {
        let cmd = parse_store(b"a1 store 1 -flags.silent (\\Seen)\r\n");

        assert_eq!(cmd.store.operation, StoreOperation::Remove);
        assert!(cmd.store.silent);
        assert_eq!(cmd.store.flags, vec!["\\Seen".to_string()]);
    }

    /// System flags fold to their canonical spelling; other flags are kept as sent.
    #[test]
    fn canonicalizes_system_flags_only() {
        let cmd = parse_store(b"a1 STORE 1 +FLAGS (\\sEEN \\DRAFT \\MyLabel)\r\n");

        assert_eq!(
            cmd.store.flags,
            vec![
                "\\Seen".to_string(),
                "\\Draft".to_string(),
                "\\MyLabel".to_string()
            ]
        );
    }

    #[test]
    fn parses_wild_sequence() {
        let cmd = parse_store(b"a1 STORE 1:* FLAGS ()\r\n");

        assert!(matches!(
            cmd.sequences[..],
            [Sequence::Range {
                start: FetchIndicator::Index(1),
                end: FetchIndicator::Wild
            }]
        ));
        assert_eq!(cmd.store.operation, StoreOperation::Replace);
        assert!(cmd.store.flags.is_empty());
    }

    #[test]
    fn rejects_missing_flags() {
        assert!(ClientCommand::parse_bytes(b"a1 STORE 1 +FLAGS\r\n").is_err());
    }

    #[test]
    fn rejects_missing_sequence_set() {
        assert!(ClientCommand::parse_bytes(b"a1 STORE +FLAGS (\\Seen)\r\n").is_err());
    }

    #[test]
    fn rejects_unknown_store_att() {
        assert!(ClientCommand::parse_bytes(b"a1 STORE 1 +LABELS (\\Seen)\r\n").is_err());
    }
}
