use std::sync::Arc;

use sqlx::{Pool, Postgres};

use crate::{
    command::{ClientCommand, ListCommand, LogoutCommand},
    response::{
        CapabilityResponse, Greeting, ListResponse, LoginResponse, LoginResult, LogoutResponse,
        MailboxListEntry, ServerResponse, ServerResponseTrait,
    },
};

pub struct IMAPSession {
    db_pool: Arc<Pool<Postgres>>,
    auth_state: SessionState,
}

#[derive(Default)]
enum SessionState {
    #[default]
    Uninitialized,
    NotAuthenticated,
    Authenticated,
    Logout,
}

impl IMAPSession {
    pub fn new(db_pool: Arc<Pool<Postgres>>) -> Self {
        Self {
            db_pool,
            auth_state: SessionState::default(),
        }
    }

    pub fn send_greeting(&mut self) -> Greeting {
        self.auth_state = SessionState::NotAuthenticated;
        Greeting::default()
    }

    pub fn is_logged_out(&self) -> bool {
        matches!(self.auth_state, SessionState::Logout)
    }

    pub fn handle_command(&mut self, command: ClientCommand) -> ServerResponse {
        match self.auth_state {
            SessionState::Uninitialized => panic!("Recevied command in an uninitialized state"),
            SessionState::NotAuthenticated => match command {
                ClientCommand::Capability(cmd) => CapabilityResponse::respond_to(cmd).into(),
                ClientCommand::StartTLS(_) => unimplemented!(),
                ClientCommand::Login(cmd) => {
                    let res = LoginResponse::respond_to(cmd);
                    match res.result {
                        LoginResult::Ok => {
                            self.auth_state = SessionState::Authenticated;
                            res.into()
                        }
                        LoginResult::No => res.into(),
                    }
                }
                ClientCommand::List(_) => todo!("This should return an error"),
                ClientCommand::Logout(cmd) => self.handle_logout_command(cmd),
            },
            SessionState::Authenticated => match command {
                ClientCommand::Capability(_) => todo!("This should return an error"),
                ClientCommand::StartTLS(_) => todo!("This should return an error"),
                ClientCommand::Login(_) => todo!("This should return an error"),
                ClientCommand::List(cmd) => self.handle_list_command(cmd),
                ClientCommand::Logout(cmd) => self.handle_logout_command(cmd),
            },
            SessionState::Logout => panic!("Received command after LOGOUT"),
        }
    }

    fn handle_logout_command(&mut self, cmd: LogoutCommand) -> ServerResponse {
        self.auth_state = SessionState::Logout;
        LogoutResponse::respond_to(cmd).into()
    }

    fn handle_list_command(&mut self, cmd: ListCommand) -> ServerResponse {
        let inbox = MailboxListEntry::new(
            vec!["\\Unmarked", "\\HasNoChildren"],
            None,
            "Inbox".to_string(),
        );
        let mailboxes = vec![inbox];
        ListResponse::new(cmd.tag, mailboxes).into()
    }
}
