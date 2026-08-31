use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
    errors::CommandParseError,
};

#[derive(Debug)]
pub struct CapabilityCommand {
    pub tag: String,
}

impl ClientCommandTrait for CapabilityCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        cursor.eat(b'\r')?;
        cursor.eat(b'\n')?;
        Ok(Self { tag })
    }
}

client_command_from_impl!(CapabilityCommand, Capability);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_capability() {
        let (cmd, read) = ClientCommand::parse_bytes(b"a1 CAPABILITY\r\n").unwrap();
        assert!(matches!(cmd, ClientCommand::Capability(_)));
        assert_eq!(read, 15);
    }

    macro_rules! tag_test {
        ($name:ident, $tag:expr) => {
            #[test]
            fn $name() {
                let tag = $tag;
                let input = format!("{tag} CAPABILITY\r\n");
                let (cmd, read) = ClientCommand::parse_bytes(input.as_bytes()).unwrap();

                assert!(matches!(&cmd, ClientCommand::Capability(c) if c.tag == tag));
                assert_eq!(read, tag.len() + 13); // tag + " CAPABILITY\r\n"
            }
        };
    }

    tag_test!(parses_capability_tag_a1, "a1");
    tag_test!(parses_capability_tag_short, "x");
    tag_test!(parses_capability_tag_long, "A0001");
}
