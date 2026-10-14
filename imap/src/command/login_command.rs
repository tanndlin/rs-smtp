use crate::{
    command::client_command::ClientCommandTrait, cursor::Cursor, errors::CommandParseError,
};

#[derive(Debug)]
pub struct LoginCommand {
    pub tag: String,
    pub user: String,
    pub pass: String,
}

impl ClientCommandTrait for LoginCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let user = cursor.string()?.to_string();
        let pass = cursor.string()?.to_string();

        cursor.expect_crlf()?;

        Ok(Self { tag, user, pass })
    }

    fn tag(&self) -> &str {
        &self.tag
    }
}

#[cfg(test)]
mod tests {
    use crate::command::ClientCommand;

    #[test]
    fn parses_login_command() {
        let (ClientCommand::Login(cmd), _) =
            ClientCommand::parse_bytes(b"a69 LOGIN \"test\" \"pass\"\r\n").unwrap()
        else {
            panic!()
        };

        assert_eq!(cmd.tag, "a69");
        assert_eq!(cmd.user, "test");
        assert_eq!(cmd.pass, "pass");
    }

    #[test]
    fn parses_unquoted_user() {
        let (ClientCommand::Login(cmd), _) =
            ClientCommand::parse_bytes(b"a69 LOGIN test \"pass\"\r\n").unwrap()
        else {
            panic!()
        };

        assert_eq!(cmd.tag, "a69");
        assert_eq!(cmd.user, "test");
        assert_eq!(cmd.pass, "pass");
    }
}
