use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
};

#[derive(Debug)]
pub struct StartTLSCommand {
    pub tag: String,
}

impl ClientCommandTrait for StartTLSCommand {
    fn with_args(tag: String, args: &[String]) -> Self {
        if args.len() > 0 {
            panic!();
        }

        Self { tag }
    }
}

client_command_from_impl!(StartTLSCommand, StartTLS);
