use crate::command::ClientCommand;

#[derive(Debug)]
pub struct CapabilityCommand {}

impl CapabilityCommand {
    pub fn new(args: String) -> Self {
        if args.len() > 0 {
            panic!();
        }

        Self {}
    }
}

impl From<CapabilityCommand> for ClientCommand {
    fn from(cmd: CapabilityCommand) -> Self {
        ClientCommand::Capability(cmd)
    }
}
