use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
};

#[derive(Debug)]
pub struct LoginCommand {
    pub tag: String,
    pub user: String,
    pub pass: String,
}

impl ClientCommandTrait for LoginCommand {
    fn with_args(tag: String, args: &[String]) -> Self {
        assert!(args.len() == 2);
        let user = args[0].trim_matches('"').to_string();
        let pass = args[1].trim_matches('"').to_string();

        Self { tag, user, pass }
    }
}

client_command_from_impl!(LoginCommand, Login);
