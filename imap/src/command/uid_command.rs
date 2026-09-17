use std::str::FromStr;

use crate::{
    client_command_from_impl,
    command::{ClientCommand, Fetchable, Sequence, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct UIDCommand {
    pub tag: String,
    pub command: UIDCommandType,
}

#[derive(Debug)]
pub enum UIDCommandType {
    Fetch {
        sequences: Vec<Sequence>,
        fetchable: Vec<Fetchable>,
    },
    Store {
        sequences: Vec<Sequence>,
        store: Store,
    },
    Copy {
        sequences: Vec<Sequence>,
        mailbox: String,
    },
}

#[derive(Debug)]
pub struct Store {
    pub operation: StoreOperation,
    pub silent: bool,
    pub flags: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StoreOperation {
    Replace,
    Add,
    Remove,
}

impl ClientCommandTrait for UIDCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let command_text = cursor.atom()?.to_uppercase();
        let command = match command_text.as_str() {
            "FETCH" => {
                let sequences = Sequence::parse_set(cursor)?;
                let fetch_list = cursor.paren_list(|c| c.fetch_att()).unwrap_or_default();
                let fetchable = fetch_list
                    .into_iter()
                    .map(Fetchable::from_str)
                    .collect::<Result<Vec<_>, _>>()?;

                UIDCommandType::Fetch {
                    sequences,
                    fetchable,
                }
            }
            "STORE" => {
                let sequences = Sequence::parse_set(cursor)?;
                let store = parse_store(cursor)?;

                UIDCommandType::Store { sequences, store }
            }
            "COPY" => {
                let sequences = Sequence::parse_set(cursor)?;
                let mailbox = cursor.string()?.to_string();

                UIDCommandType::Copy { sequences, mailbox }
            }
            _ => {
                return Err(CommandParseError::MalformedCommand(Some(format!(
                    "UID: unsupported command {command_text:?}"
                ))));
            }
        };

        cursor.expect_crlf()?;

        Ok(Self { tag, command })
    }

    fn tag(&self) -> &str {
        &self.tag
    }
}

/// Parse a `store-att-flags`: the operation, its optional `.SILENT` suffix and
/// the flags to apply.
fn parse_store(cursor: &mut Cursor) -> Result<Store, CommandParseError> {
    // `+`, `-` and `.` are all atom chars, so the operation, `FLAGS` and
    // `.SILENT` come back as one atom.
    let att = cursor.atom()?.to_uppercase();
    let (operation, flags_att) = match att.as_bytes().first() {
        Some(b'+') => (StoreOperation::Add, &att[1..]),
        Some(b'-') => (StoreOperation::Remove, &att[1..]),
        _ => (StoreOperation::Replace, att.as_str()),
    };

    let silent = match flags_att {
        "FLAGS" => false,
        "FLAGS.SILENT" => true,
        _ => {
            return Err(CommandParseError::MalformedCommand(Some(format!(
                "UID STORE: expected FLAGS or FLAGS.SILENT, got {att:?}"
            ))));
        }
    };

    let flags = if cursor.peek_nonspace() == Some(b'(') {
        // `flag-list`, which may be empty - `FLAGS ()` clears every flag.
        cursor.paren_list(|c| c.flag().map(|f| f.to_string()))?
    } else {
        // `flag *(SP flag)`. `paren_list` can't take this form because a flag
        // starts with `\`, which is not an atom char.
        let mut flags = vec![cursor.flag()?.to_string()];
        while cursor.peek_nonspace() == Some(b'\\') {
            flags.push(cursor.flag()?.to_string());
        }

        flags
    };

    Ok(Store {
        operation,
        silent,
        flags,
    })
}

client_command_from_impl!(UIDCommand, UID);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::FetchIndicator;

    fn parse_uid(buf: &[u8]) -> UIDCommandType {
        let (cmd, _) = ClientCommand::parse_bytes(buf).unwrap();
        let ClientCommand::UID(cmd) = cmd else {
            panic!("expected ClientCommand::UID, got {cmd:?}");
        };

        cmd.command
    }

    /// `+FLAGS` adds to the existing flags and is not silent.
    #[test]
    fn parses_add_flags() {
        let UIDCommandType::Store { sequences, store } =
            parse_uid(b"a1 UID STORE 90210 +FLAGS (\\Deleted)\r\n")
        else {
            panic!("expected a store");
        };

        assert!(matches!(
            sequences[..],
            [Sequence::Single(FetchIndicator::Index(90210))]
        ));
        assert_eq!(store.operation, StoreOperation::Add);
        assert!(!store.silent);
        assert_eq!(store.flags, vec!["\\DELETED".to_string()]);
    }

    /// `-FLAGS.SILENT` removes flags and suppresses the untagged FETCHes.
    #[test]
    fn parses_remove_flags_silent() {
        let UIDCommandType::Store { store, .. } =
            parse_uid(b"a1 UID STORE 1:* -FLAGS.SILENT (\\Seen \\Flagged)\r\n")
        else {
            panic!("expected a store");
        };

        assert_eq!(store.operation, StoreOperation::Remove);
        assert!(store.silent);
        assert_eq!(
            store.flags,
            vec!["\\SEEN".to_string(), "\\FLAGGED".to_string()]
        );
    }

    /// A bare `FLAGS` replaces the whole flag set.
    #[test]
    fn parses_replace_flags() {
        let UIDCommandType::Store { store, .. } =
            parse_uid(b"a1 UID STORE 1,3:5 FLAGS (\\Seen)\r\n")
        else {
            panic!("expected a store");
        };

        assert_eq!(store.operation, StoreOperation::Replace);
        assert!(!store.silent);
        assert_eq!(store.flags, vec!["\\SEEN".to_string()]);
    }

    /// `FLAGS ()` is legal and clears every flag.
    #[test]
    fn parses_empty_flag_list() {
        let UIDCommandType::Store { store, .. } = parse_uid(b"a1 UID STORE 7 FLAGS ()\r\n") else {
            panic!("expected a store");
        };

        assert_eq!(store.operation, StoreOperation::Replace);
        assert!(store.flags.is_empty());
    }

    /// The flags may also come unparenthesised.
    #[test]
    fn parses_bare_flags() {
        let UIDCommandType::Store { store, .. } =
            parse_uid(b"a1 UID STORE 7 +FLAGS \\Seen \\Answered\r\n")
        else {
            panic!("expected a store");
        };

        assert_eq!(
            store.flags,
            vec!["\\SEEN".to_string(), "\\ANSWERED".to_string()]
        );
    }

    /// The sequence set is a full set, not a single number.
    #[test]
    fn parses_copy_sequence_set() {
        let UIDCommandType::Copy { sequences, mailbox } =
            parse_uid(b"a1 UID COPY 1,3:5 \"Sent Mail\"\r\n")
        else {
            panic!("expected a copy");
        };

        assert_eq!(sequences.len(), 2);
        assert!(matches!(
            sequences[0],
            Sequence::Single(FetchIndicator::Index(1))
        ));
        assert!(matches!(
            sequences[1],
            Sequence::Range {
                start: FetchIndicator::Index(3),
                end: FetchIndicator::Index(5)
            }
        ));
        assert_eq!(mailbox, "Sent Mail");
    }

    /// An unquoted mailbox name is an atom.
    #[test]
    fn parses_copy_atom_mailbox() {
        let UIDCommandType::Copy { mailbox, .. } = parse_uid(b"a1 UID COPY 7 INBOX\r\n") else {
            panic!("expected a copy");
        };

        assert_eq!(mailbox, "INBOX");
    }

    /// `STORE` needs an operation we understand.
    #[test]
    fn rejects_unknown_store_att() {
        assert!(ClientCommand::parse_bytes(b"a1 UID STORE 1 +LABELS (\\Seen)\r\n").is_err());
    }

    /// Only FETCH / STORE / COPY are wired up so far.
    #[test]
    fn rejects_unsupported_uid_command() {
        assert!(ClientCommand::parse_bytes(b"a1 UID EXPUNGE 1\r\n").is_err());
    }
}
