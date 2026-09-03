use std::{collections::HashMap, fmt, str::FromStr};

use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct FetchCommand {
    pub tag: String,
    pub sequences: Vec<Sequence>,
    pub fetch_list: Vec<Fetchable>,
}

#[derive(Debug)]
pub enum Sequence {
    Single(FetchIndicator),
    Range {
        start: FetchIndicator,
        end: FetchIndicator,
    },
}

// TODO: I hate this API
impl Sequence {
    pub fn single_to_message_ids(&self, last: u64) -> Vec<u64> {
        fn resolve(indicator: &FetchIndicator, last: u64) -> u64 {
            match indicator {
                FetchIndicator::Index(i) => *i,
                FetchIndicator::Wild => last,
            }
        }

        match self {
            Sequence::Single(indicator) => match resolve(indicator, last) {
                n if (1..=last).contains(&n) => vec![n],
                _ => vec![],
            },
            Sequence::Range { start, end } => {
                let a = resolve(start, last);
                let b = resolve(end, last);
                let lo = a.min(b).max(1);
                let hi = a.max(b).min(last);
                (lo..=hi).collect()
            }
        }
    }

    pub fn to_message_ids(sequences: &[Self], last: u64) -> Vec<u64> {
        let mut ids: Vec<u64> = sequences
            .iter()
            .flat_map(|sequence| sequence.single_to_message_ids(last))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }
}

#[derive(Debug)]
pub enum FetchIndicator {
    Index(u64),
    Wild,
}

impl FromStr for FetchIndicator {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "*" {
            return Ok(Self::Wild);
        }

        dbg!(&s);

        Ok(FetchIndicator::Index(s.parse()?))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Fetchable {
    All,
    Fast,
    Full,
    Binary(BinaryFetchable),
    Body(BodyFetchable),
    BodyStructure,
    Envelope,
    Flags,
    Internaldate,
    RFC822Size,
    UID,
}

impl FromStr for Fetchable {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let matches = HashMap::from([
            (
                "ALL",
                (|_: &str| Ok(Self::All)) as fn(&str) -> Result<Self, Self::Err>,
            ),
            ("FAST", |_| Ok(Self::Fast)),
            ("FULL", |_| Ok(Self::Full)),
            ("BINARY", |s| {
                Ok(Fetchable::Binary(BinaryFetchable::from_str(s)?))
            }),
            ("BODY", |s| Ok(Fetchable::Body(BodyFetchable::from_str(s)?))),
            ("BODYSTRUCTURE", |_| Ok(Self::BodyStructure)),
            ("ENVELOPE", |_| Ok(Self::Envelope)),
            ("FLAGS", |_| Ok(Self::Flags)),
            ("INTERNALDATE", |_| Ok(Self::Internaldate)),
            ("RFC822.SIZE", |_| Ok(Self::RFC822Size)),
            ("UID", |_| Ok(Self::UID)),
        ]);

        matches
            .keys()
            .filter(|key| s.starts_with(**key))
            .max_by_key(|key| key.len())
            .ok_or(CommandParseError::MalformedCommand(Some(format!(
                "Fetchable: unknown attribute {s:?}"
            ))))
            .map(|key| matches[key](s))?
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BinaryFetchable {
    Content {
        peek: bool,
        part: Vec<u32>,
        partial: Option<Partial>,
    },
    Size {
        part: Vec<u32>,
    },
}

impl FromStr for BinaryFetchable {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rest = s.strip_prefix("BINARY").unwrap();

        if let Some(rest) = rest.strip_prefix(".SIZE") {
            return Ok(BinaryFetchable::Size {
                part: parse_section_binary(rest)?,
            });
        }

        let (peek, rest) = match rest.strip_prefix(".PEEK") {
            Some(rest) => (true, rest),
            None => (false, rest),
        };

        let rest = rest
            .strip_prefix('[')
            .ok_or(CommandParseError::MalformedCommand(Some(format!(
                "BinaryFetchable: Expected \"[\". Got: {rest}"
            ))))?;
        let close = rest
            .find(']')
            .ok_or(CommandParseError::MalformedCommand(Some(format!(
                "BinaryFetchable: Expected \"]\". Got: {rest}"
            ))))?;

        let inner = rest[..close].trim();
        let part = if inner.is_empty() {
            Vec::new()
        } else {
            let (nums, leftover) = split_part_numbers(inner);
            // BINARY only addresses numbered parts - no HEADER/TEXT/MIME.
            if !leftover.is_empty() {
                return Err(CommandParseError::MalformedCommand(Some(format!(
                    "BinaryFetchable: left over parts: {leftover}"
                ))));
            }
            nums
        };

        let after = &rest[close + 1..];
        let partial = if after.is_empty() {
            None
        } else {
            Some(Partial::from_str(after)?)
        };

        Ok(BinaryFetchable::Content {
            peek,
            part,
            partial,
        })
    }
}

/// Renders a `Fetchable` as the message-data-item name a FETCH *response*
/// uses (RFC 9051 §7.5.2), i.e. the key half of an `att value` pair. The
/// `.PEEK` request modifier is dropped — a response is always the plain
/// `BODY[...]` form.
impl fmt::Display for Fetchable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fetchable::All => f.write_str("ALL"),
            Fetchable::Fast => f.write_str("FAST"),
            Fetchable::Full => f.write_str("FULL"),
            Fetchable::Binary(binary) => write!(f, "{binary}"),
            Fetchable::Body(body) => write!(f, "{body}"),
            Fetchable::BodyStructure => f.write_str("BODYSTRUCTURE"),
            Fetchable::Envelope => f.write_str("ENVELOPE"),
            Fetchable::Flags => f.write_str("FLAGS"),
            Fetchable::Internaldate => f.write_str("INTERNALDATE"),
            Fetchable::RFC822Size => f.write_str("RFC822.SIZE"),
            Fetchable::UID => f.write_str("UID"),
        }
    }
}

impl fmt::Display for BodyFetchable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BodyFetchable::Full => f.write_str("BODY"),
            BodyFetchable::Section {
                peek: _,
                section,
                partial,
            } => {
                write!(f, "BODY[{section}]")?;
                if let Some(partial) = partial {
                    write!(f, "{partial}")?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for BinaryFetchable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryFetchable::Size { part } => write!(f, "BINARY.SIZE[{}]", join_parts(part)),
            BinaryFetchable::Content {
                peek: _,
                part,
                partial,
            } => {
                write!(f, "BINARY[{}]", join_parts(part))?;
                if let Some(partial) = partial {
                    write!(f, "{partial}")?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Section::Full => Ok(()),
            Section::Msg(text) => write!(f, "{text}"),
            Section::Part { part, text } => {
                write!(f, "{}", join_parts(part))?;
                if let Some(text) = text {
                    write!(f, ".{text}")?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for SectionText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SectionText::Header => f.write_str("HEADER"),
            SectionText::HeaderFields(fields) => write!(f, "HEADER.FIELDS ({})", fields.join(" ")),
            SectionText::HeaderFieldsNot(fields) => {
                write!(f, "HEADER.FIELDS.NOT ({})", fields.join(" "))
            }
            SectionText::Text => f.write_str("TEXT"),
            SectionText::Mime => f.write_str("MIME"),
        }
    }
}

impl fmt::Display for Partial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}.{}>", self.start, self.count)
    }
}

/// Join a section-part path back into its dotted form: `[1, 2, 3]` -> `"1.2.3"`.
fn join_parts(parts: &[u32]) -> String {
    parts
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

/// Parse a `[section-part]` (numbered parts only, e.g. `[1.2]` or `[]`).
fn parse_section_binary(s: &str) -> Result<Vec<u32>, CommandParseError> {
    let inner = s
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or(CommandParseError::MalformedCommand(Some(format!(
            "Binary section parser: Missing brackets. Got: {s}"
        ))))?
        .trim();

    if inner.is_empty() {
        return Ok(Vec::new());
    }

    let (nums, leftover) = split_part_numbers(inner);
    if leftover.is_empty() {
        Ok(nums)
    } else {
        Err(CommandParseError::MalformedCommand(Some(format!(
            "Binary section parser: left over parts: {leftover}"
        ))))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BodyFetchable {
    /// `BODY` on its own - the non-extensible form of `BODYSTRUCTURE`.
    Full,
    /// `BODY[<section>]` / `BODY.PEEK[<section>]`, with an optional
    /// `<start.count>` partial. `peek` is set for the `.PEEK` form.
    Section {
        peek: bool,
        section: Section,
        partial: Option<Partial>,
    },
}

impl FromStr for BodyFetchable {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rest = s.strip_prefix("BODY").unwrap();

        if rest.is_empty() {
            return Ok(BodyFetchable::Full);
        }

        let (peek, rest) = match rest.strip_prefix(".PEEK") {
            Some(rest) => (true, rest),
            None => (false, rest),
        };

        let rest = rest
            .strip_prefix('[')
            .ok_or(CommandParseError::MalformedCommand(Some(format!(
                "BodyFetchable: Expected \"[\". Got: {rest}"
            ))))?;
        let close = rest
            .find(']')
            .ok_or(CommandParseError::MalformedCommand(Some(format!(
                "BodyFetchable: Expected \"]\". Got: {rest}"
            ))))?;
        let section = parse_section(rest[..close].trim())?;

        let after = &rest[close + 1..];
        let partial = if after.is_empty() {
            None
        } else {
            Some(Partial::from_str(after)?)
        };

        Ok(BodyFetchable::Section {
            peek,
            section,
            partial,
        })
    }
}

/// A `BODY[...]` section specifier.
#[derive(Debug, PartialEq, Eq)]
pub enum Section {
    /// `[]` - the entire message.
    Full,
    /// A message-level text spec: `[HEADER]`, `[HEADER.FIELDS (...)]`,
    /// `[HEADER.FIELDS.NOT (...)]` or `[TEXT]`.
    Msg(SectionText),
    /// A numbered body part, e.g. `[1.2.3]`, optionally with a nested
    /// text spec such as `[1.2.HEADER]` or `[4.MIME]`.
    Part {
        part: Vec<u32>,
        text: Option<SectionText>,
    },
}

/// The text portion of a section spec, shared by the message-level and
/// per-part forms. `Mime` is only meaningful after a part number.
#[derive(Debug, PartialEq, Eq)]
pub enum SectionText {
    Header,
    HeaderFields(Vec<String>),
    HeaderFieldsNot(Vec<String>),
    Text,
    Mime,
}

fn parse_section(inner: &str) -> Result<Section, CommandParseError> {
    if inner.is_empty() {
        return Ok(Section::Full);
    }

    if inner.starts_with(|c: char| c.is_ascii_digit()) {
        let (part, rest) = split_part_numbers(inner);
        let text = if rest.is_empty() {
            None
        } else {
            Some(parse_section_text(rest)?)
        };
        return Ok(Section::Part { part, text });
    }

    Ok(Section::Msg(parse_section_text(inner)?))
}

fn parse_section_text(s: &str) -> Result<SectionText, CommandParseError> {
    let upper = s.trim().to_ascii_uppercase();

    match upper.as_str() {
        "HEADER" => return Ok(SectionText::Header),
        "TEXT" => return Ok(SectionText::Text),
        "MIME" => return Ok(SectionText::Mime),
        _ => {}
    }

    if let Some(list) = upper.strip_prefix("HEADER.FIELDS.NOT") {
        return Ok(SectionText::HeaderFieldsNot(parse_header_list(list)?));
    }
    if let Some(list) = upper.strip_prefix("HEADER.FIELDS") {
        return Ok(SectionText::HeaderFields(parse_header_list(list)?));
    }

    Err(CommandParseError::MalformedCommand(Some(format!(
        "SectionText: unknown text spec {s:?}"
    ))))
}

/// Parse a parenthesised, space-separated `(Field-1 Field-2 ...)` list.
fn parse_header_list(s: &str) -> Result<Vec<String>, CommandParseError> {
    let inner = s
        .trim()
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or(CommandParseError::MalformedCommand(Some(format!(
            "Header field list: Expected \"(...)\". Got: {s}"
        ))))?;

    let fields: Vec<String> = inner.split_whitespace().map(str::to_string).collect();
    if fields.is_empty() {
        return Err(CommandParseError::MalformedCommand(Some(format!(
            "Header field list: empty list. Got: {s}"
        ))));
    }
    Ok(fields)
}

/// Split a leading run of dot-separated integers (`1.2.3`) from any
/// trailing text specifier. Returns `(numbers, rest)` where `rest` is
/// what follows the last number (leading `.` removed), or `""`.
fn split_part_numbers(s: &str) -> (Vec<u32>, &str) {
    let mut nums = Vec::new();
    let mut rest = s;

    loop {
        let seg_end = rest.find('.').unwrap_or(rest.len());
        match rest[..seg_end].parse::<u32>() {
            Ok(n) => {
                nums.push(n);
                if seg_end == rest.len() {
                    return (nums, "");
                }
                rest = &rest[seg_end + 1..];
            }
            Err(_) => return (nums, rest),
        }
    }
}

/// `<start.count>` partial specifier on a `BODY[...]` / `BINARY[...]` item.
#[derive(Debug, PartialEq, Eq)]
pub struct Partial {
    pub start: u64,
    pub count: u64,
}

impl FromStr for Partial {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = s
            .strip_prefix('<')
            .and_then(|s| s.strip_suffix('>'))
            .ok_or(CommandParseError::MalformedCommand(Some(format!(
                "Partial: Expected \"<...>\". Got: {s}"
            ))))?;

        let (start, count) = inner
            .split_once('.')
            .ok_or(CommandParseError::MalformedCommand(Some(format!(
                "Partial: No . delimiter found. Got: {inner}"
            ))))?;
        Ok(Partial {
            start: start.parse()?,
            count: count.parse()?,
        })
    }
}

impl ClientCommandTrait for FetchCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let sequences = {
            let mut sequences = vec![];
            loop {
                let start = cursor.sequence_indicator()?;
                dbg!(&start);
                if let Some(next) = cursor.peek_nonspace()
                    && next == b':'
                {
                    cursor.eat(b':').unwrap();
                    let end = cursor.sequence_indicator()?;
                    dbg!(&end);
                    sequences.push(Sequence::Range { start, end })
                } else {
                    sequences.push(Sequence::Single(start))
                }

                if let Some(next) = cursor.peek_nonspace()
                    && next != b','
                {
                    break;
                }
                cursor.eat(b',').unwrap();
            }

            dbg!(&sequences);
            sequences
        };

        // A FETCH takes either a parenthesized list of items, or a single
        // bare item with no parens (e.g. `FETCH 1 BODY`).
        let items = match cursor.paren_list(|c| c.fetch_att()) {
            Ok(items) => items,
            Err(_) => vec![cursor.fetch_att()?],
        };

        // TODO: Technically a macro cannot be in a paren list but i dont feel like enforcing that rn
        let fetch_list = items
            .into_iter()
            .map(Fetchable::from_str)
            .collect::<Result<Vec<_>, _>>()?;

        if (fetch_list.contains(&Fetchable::All)
            || fetch_list.contains(&Fetchable::Full)
            || fetch_list.contains(&Fetchable::Fast))
            && fetch_list.len() != 1
        {
            return Err(CommandParseError::MalformedCommand(Some(format!(
                "FETCH Command can only have 1 macro: Got: {fetch_list:?}"
            ))));
        }

        Ok(Self {
            tag,
            sequences,
            fetch_list,
        })
    }

    fn tag(&self) -> &str {
        &self.tag
    }
}

client_command_from_impl!(FetchCommand, Fetch);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_fetch() {
        let (cmd, _) = ClientCommand::parse_bytes(b"a1 FETCH 1 BODY\r\n").unwrap();
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        assert!(cmd.sequences.len() == 1);
        assert!(matches!(
            cmd.sequences[0],
            Sequence::Single(FetchIndicator::Index(1))
        ));
        assert!(cmd.fetch_list.len() == 1);
        assert!(matches!(
            cmd.fetch_list[0],
            Fetchable::Body(BodyFetchable::Full)
        ));
    }

    #[test]
    fn parses_single_sequence() {
        let (cmd, _) = ClientCommand::parse_bytes(b"a1 FETCH 1 BODY\r\n").unwrap();
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        assert!(cmd.sequences.len() == 1);
        assert!(matches!(
            cmd.sequences[0],
            Sequence::Single(FetchIndicator::Index(1))
        ));
        assert!(cmd.fetch_list.len() == 1);
        assert!(matches!(
            cmd.fetch_list[0],
            Fetchable::Body(BodyFetchable::Full)
        ));
    }

    #[test]
    fn parses_single_wild() {
        let (cmd, _) = ClientCommand::parse_bytes(b"a1 FETCH * BODY\r\n").unwrap();
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        assert!(cmd.sequences.len() == 1);
        assert!(matches!(
            cmd.sequences[0],
            Sequence::Single(FetchIndicator::Wild)
        ));
        assert!(cmd.fetch_list.len() == 1);
        assert!(matches!(
            cmd.fetch_list[0],
            Fetchable::Body(BodyFetchable::Full)
        ));
    }

    #[test]
    fn parses_sequence_wild() {
        let buf = b"a1 FETCH 1:* BODY\r\n";
        let cmd = match ClientCommand::parse_bytes(buf) {
            Ok((cmd, _)) => cmd,
            Err(e) => {
                eprintln!("{}", e.render(buf));
                panic!()
            }
        };
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        assert!(matches!(
            cmd.sequences[0],
            Sequence::Range {
                start: FetchIndicator::Index(1),
                end: FetchIndicator::Wild
            }
        ));

        assert!(cmd.fetch_list.len() == 1);
        assert!(matches!(
            cmd.fetch_list[0],
            Fetchable::Body(BodyFetchable::Full)
        ));
    }

    #[test]
    fn parses_double_index_range() {
        let (cmd, _) = ClientCommand::parse_bytes(b"a1 FETCH 1:9 BODY\r\n").unwrap();
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        assert!(matches!(
            cmd.sequences[0],
            Sequence::Range {
                start: FetchIndicator::Index(1),
                end: FetchIndicator::Index(9)
            }
        ));

        assert!(cmd.fetch_list.len() == 1);
        assert!(matches!(
            cmd.fetch_list[0],
            Fetchable::Body(BodyFetchable::Full)
        ));
    }

    #[test]
    fn body_bare_is_full() {
        let (cmd, _) = ClientCommand::parse_bytes(b"a1 FETCH 1 BODY\r\n").unwrap();
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        let Fetchable::Body(body) = &cmd.fetch_list[0] else {
            panic!("expected Fetchable::Body, got {:?}", cmd.fetch_list[0]);
        };
        assert_eq!(*body, BodyFetchable::Full);
    }

    #[test]
    fn body_whole_message_with_partial() {
        let (cmd, _) = ClientCommand::parse_bytes(b"a1 FETCH 1 BODY[]<0.2048>\r\n").unwrap();
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        let Fetchable::Body(body) = &cmd.fetch_list[0] else {
            panic!("expected Fetchable::Body, got {:?}", cmd.fetch_list[0]);
        };
        assert_eq!(
            *body,
            BodyFetchable::Section {
                peek: false,
                section: Section::Full,
                partial: Some(Partial {
                    start: 0,
                    count: 2048
                }),
            }
        );
    }

    #[test]
    fn body_peek_header_fields() {
        let (cmd, _) = ClientCommand::parse_bytes(
            b"a1 FETCH 1 BODY.PEEK[HEADER.FIELDS (DATE FROM SUBJECT)]\r\n",
        )
        .unwrap();
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        let Fetchable::Body(body) = &cmd.fetch_list[0] else {
            panic!("expected Fetchable::Body, got {:?}", cmd.fetch_list[0]);
        };
        assert_eq!(
            *body,
            BodyFetchable::Section {
                peek: true,
                section: Section::Msg(SectionText::HeaderFields(vec![
                    "DATE".into(),
                    "FROM".into(),
                    "SUBJECT".into(),
                ])),
                partial: None,
            }
        );
    }

    #[test]
    fn body_nested_part_mime() {
        let (cmd, _) = ClientCommand::parse_bytes(b"a1 FETCH 1 BODY[4.1.MIME]\r\n").unwrap();
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        let Fetchable::Body(body) = &cmd.fetch_list[0] else {
            panic!("expected Fetchable::Body, got {:?}", cmd.fetch_list[0]);
        };
        assert_eq!(
            *body,
            BodyFetchable::Section {
                peek: false,
                section: Section::Part {
                    part: vec![4, 1],
                    text: Some(SectionText::Mime),
                },
                partial: None,
            }
        );
    }

    #[test]
    fn body_part_no_text() {
        let (cmd, _) = ClientCommand::parse_bytes(b"a1 FETCH 1 BODY[2]\r\n").unwrap();
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        let Fetchable::Body(body) = &cmd.fetch_list[0] else {
            panic!("expected Fetchable::Body, got {:?}", cmd.fetch_list[0]);
        };
        assert_eq!(
            *body,
            BodyFetchable::Section {
                peek: false,
                section: Section::Part {
                    part: vec![2],
                    text: None,
                },
                partial: None,
            }
        );
    }

    #[test]
    fn body_peek_without_section_is_err() {
        assert!(ClientCommand::parse_bytes(b"a1 FETCH 1 BODY.PEEK\r\n").is_err());
    }

    #[test]
    fn binary_peek_with_partial() {
        let (cmd, _) =
            ClientCommand::parse_bytes(b"a1 FETCH 1 BINARY.PEEK[1.2]<0.512>\r\n").unwrap();
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        let Fetchable::Binary(binary) = &cmd.fetch_list[0] else {
            panic!("expected Fetchable::Binary, got {:?}", cmd.fetch_list[0]);
        };
        assert_eq!(
            *binary,
            BinaryFetchable::Content {
                peek: true,
                part: vec![1, 2],
                partial: Some(Partial {
                    start: 0,
                    count: 512
                }),
            }
        );
    }

    #[test]
    fn binary_size() {
        let (cmd, _) = ClientCommand::parse_bytes(b"a1 FETCH 1 BINARY.SIZE[3]\r\n").unwrap();
        let ClientCommand::Fetch(cmd) = cmd else {
            panic!("expected ClientCommand::Fetch, got {cmd:?}");
        };

        let Fetchable::Binary(binary) = &cmd.fetch_list[0] else {
            panic!("expected Fetchable::Binary, got {:?}", cmd.fetch_list[0]);
        };
        assert_eq!(*binary, BinaryFetchable::Size { part: vec![3] });
    }

    #[test]
    fn binary_rejects_text_section() {
        assert!(ClientCommand::parse_bytes(b"a1 FETCH 1 BINARY[HEADER]\r\n").is_err());
    }
}
