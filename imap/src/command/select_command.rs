use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
    cursor::Cursor,
};

#[derive(Debug)]
pub struct SelectCommand {
    pub tag: String,
    pub mailbox: String,
}

impl ClientCommandTrait for SelectCommand {
    fn parse_bytes(tag: String, cursor: &mut Cursor) -> Self {
        let mailbox = cursor.atom().unwrap().to_string();

        cursor.eat(b'\r');
        cursor.eat(b'\n');

        Self { tag, mailbox }
    }
}

client_command_from_impl!(SelectCommand, Select);

#[cfg(test)]
mod tests {
    use super::*;
}
