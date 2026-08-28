use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
};

#[derive(Debug)]
pub struct ListCommand {
    pub tag: String,
}

impl ClientCommandTrait for ListCommand {
    // TODO: Actually parse list
    fn with_args(tag: String, args: &[String]) -> Self {
        Self { tag }
    }
}

client_command_from_impl!(ListCommand, List);
