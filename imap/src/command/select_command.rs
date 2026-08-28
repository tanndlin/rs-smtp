use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
};

#[derive(Debug)]
pub struct SelectCommand {
    pub tag: String,
    pub mailbox: String,
}

impl ClientCommandTrait for SelectCommand {
    fn with_args(tag: String, args: &[String]) -> Self {
        dbg!(args);
        assert!(args.len() == 1);
        let mailbox = args[0].trim_matches('"').to_string();

        Self { tag, mailbox }
    }
}

client_command_from_impl!(SelectCommand, Select);

#[cfg(test)]
mod tests {
    use super::*;
}
