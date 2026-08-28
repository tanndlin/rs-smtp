use crate::{
    command::{CapabilityCommand, ListCommand, LoginCommand, LogoutCommand, StartTLSCommand},
    response::CommandParseError,
};

#[derive(Debug)]
pub enum ClientCommand {
    Capability(CapabilityCommand),
    StartTLS(StartTLSCommand),
    Login(LoginCommand),
    List(ListCommand),
    Logout(LogoutCommand),
}

impl ClientCommand {
    pub fn parse_bytes(buf: &[u8]) -> Result<(Self, usize), CommandParseError> {
        let str = str::from_utf8(buf).unwrap();
        dbg!(str);
        let lines = str.split("\r\n").collect::<Vec<_>>();
        let line = lines[0];
        println!("Got line: {:?}", line);

        let mut split = line.splitn(3, " ");
        let tag = split
            .next()
            .ok_or(CommandParseError::MalformedCommand)?
            .to_string();
        let command_text = split.next().ok_or(CommandParseError::MalformedCommand)?;
        let rest = split
            .next()
            .unwrap_or_default()
            .split(" ")
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let bytes_read = line.len() + 2; // +2 for the \r\n. TODO: This might not always be the case

        dbg!(&command_text);

        let cmd = match command_text {
            "CAPABILITY" => CapabilityCommand::with_args(tag, &rest).into(),
            "STARTTLS" => StartTLSCommand::with_args(tag, &rest).into(),
            "LOGIN" => LoginCommand::with_args(tag, &rest).into(),
            "LIST" => ListCommand::with_args(tag, &rest).into(),
            "LOGOUT" => LogoutCommand::with_args(tag, &rest).into(),
            _ => todo!("Probably havent implemented {command_text} yet"),
        };

        Ok((cmd, bytes_read))
    }
}

pub trait ClientCommandTrait {
    fn with_args(tag: String, args: &[String]) -> Self;
}

#[macro_export]
macro_rules! client_command_from_impl {
    ($type: tt,$variant: ident) => {
        impl From<$type> for ClientCommand {
            fn from(cmd: $type) -> Self {
                ClientCommand::$variant(cmd)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_tag_does_not_panic() {
        assert!(ClientCommand::parse_bytes(b"a1\r\n").is_err());
    }
}
