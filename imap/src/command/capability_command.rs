use crate::command::{ClientCommand, client_command::ClientCommandTrait};

#[derive(Debug)]
pub struct CapabilityCommand {
    pub tag: String,
}

impl ClientCommandTrait for CapabilityCommand {
    fn with_args(tag: String, args: &[String]) -> Self {
        dbg!(args);
        if !args.is_empty() {
            panic!();
        }

        Self { tag }
    }
}

impl From<CapabilityCommand> for ClientCommand {
    fn from(cmd: CapabilityCommand) -> Self {
        ClientCommand::Capability(cmd)
    }
}
