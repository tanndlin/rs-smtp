use crate::{command::ClientCommand, response::ServerResponse};

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

    pub fn send_greeting(&mut self) {
        *self = Self::NotAuthenticated
    }

    pub fn handle_command(&mut self, command: ClientCommand) -> ServerResponse {
        match command {
            ClientCommand::Capability(cmd) => ServerResponse::new_capability(),
        }
    }
}
