use crate::{
    command::ClientCommand,
    response::{
        CapabilityResponse, Greeting, LoginResponse, LoginResult, ServerResponse,
        ServerResponseTrait,
    },
};

#[derive(Default)]
pub enum IMAPState {
    #[default]
    Uninitialized,
    NotAuthenticated,
    Authenticated,
    Logout,
}

impl IMAPState {
    pub fn send_greeting(&mut self) -> Greeting {
        *self = Self::NotAuthenticated;
        Greeting::new()
    }

    pub fn handle_command(&mut self, command: ClientCommand) -> ServerResponse {
        match self {
            IMAPState::Uninitialized => panic!("Recevied command in an uninitialized state"),
            IMAPState::NotAuthenticated => match command {
                ClientCommand::Capability(cmd) => CapabilityResponse::respond_to(cmd).into(),
                ClientCommand::StartTLS(_) => unimplemented!(),
                ClientCommand::Login(cmd) => {
                    let res = LoginResponse::respond_to(cmd);
                    match res.result {
                        LoginResult::Ok => {
                            *self = IMAPState::Authenticated;
                            res.into()
                        }
                        LoginResult::No => res.into(),
                    }
                }
            },
            IMAPState::Authenticated => todo!(),
            IMAPState::Logout => todo!(),
        }
    }
}
