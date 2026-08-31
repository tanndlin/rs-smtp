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
        let mailbox = cursor.atom().unwrap().to_string();

        let flags = cursor.paren_list(|c| c.atom()).unwrap();
        let messages = flags.contains(&"MESSAGES");
        let next_uid = flags.contains(&"UIDNEXT");
        let validity_uid = flags.contains(&"UIDVALIDITY");
        let unseen = flags.contains(&"UNSEEN");
        let deleted = flags.contains(&"DELETED");
        let size = flags.contains(&"SIZE");

        cursor.eat(b'\r');
        cursor.eat(b'\n');
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
}

client_command_from_impl!(StatusCommand, Status);
