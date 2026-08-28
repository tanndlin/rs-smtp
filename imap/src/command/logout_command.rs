use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
};

#[derive(Debug)]
pub struct LogoutCommand {
    pub tag: String,
}

impl ClientCommandTrait for LogoutCommand {
    fn with_args(tag: String, args: &[String]) -> Self {
        assert!(args.is_empty());

        Self { tag }
    }
}

client_command_from_impl!(LogoutCommand, Logout);
