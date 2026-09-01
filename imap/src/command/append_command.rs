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
    pub message: Option<Vec<u8>>,
}

impl ClientCommandTrait for AppendCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Result<Self, CommandParseError> {
        let mailbox = cursor.string().unwrap().to_string();

        // TODO: Any error will assume it meant empty flags
        let flags = cursor
            .paren_list(|c| c.atom().map(|s| s.to_string()))
            .unwrap_or_default();

        let date_time = match cursor.string() {
            Ok(date_time) => Some(date_time.to_string()),
            Err(_) => None,
        };

        let (length, sending_now) = {
            let length = cursor
                .atom()
                .unwrap()
                .trim_start_matches('{')
                .trim_end_matches('}');
            let sending_now = length.contains('+');
            let length = length.trim_end_matches('+').parse().unwrap();
            (length, sending_now)
        };

        cursor.eat(b'\r').unwrap();
        cursor.eat(b'\n').unwrap();

        let message = if sending_now {
            let message = cursor.raw(length);
            cursor.eat(b'\r').unwrap();
            cursor.eat(b'\n').unwrap();
            Some(message)
        } else {
            None
        };

        Ok(Self {
            tag,
            mailbox,
            flags,
            date_time,
            message,
        })
    }
}

client_command_from_impl!(AppendCommand, Append);
