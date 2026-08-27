use crate::{
    command::ClientCommand,
    response::{CapabilityResponse, Greeting, ServerResponse, ServerResponseTrait},
};

pub enum IMAPState {
    Uninitialized,
    NotAuthenticated,
    Authenticated,
    Logout,
}

impl IMAPState {
    pub fn new() -> Self {
        Self::Uninitialized
    }

    pub fn send_greeting(&mut self) -> Greeting {
        *self = Self::NotAuthenticated;
        Greeting::new()
    }

    pub fn handle_command(&mut self, command: ClientCommand) -> ServerResponse {
        match self {
            IMAPState::Uninitialized => panic!("Recevied command in an uninitialized state"),
            IMAPState::NotAuthenticated => match command {
                ClientCommand::Capability(cmd) => CapabilityResponse::respond_to(cmd).into(),
                ClientCommand::StartTLS(cmd) => todo!(),
            },
            IMAPState::Authenticated => todo!(),
            IMAPState::Logout => todo!(),
        }
    }
}
