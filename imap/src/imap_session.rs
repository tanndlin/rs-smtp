use std::sync::Arc;

use sqlx::{Pool, Postgres};

use crate::{
    command::{ClientCommand, ListCommand, LogoutCommand, SelectCommand},
    response::{
        CapabilityResponse, Greeting, ListResponse, LoginResponse, LoginResult, LogoutResponse,
        MailboxListEntry, SelectResponse, ServerResponse, ServerResponseTrait,
    },
};

pub struct IMAPSession {
    db_pool: Arc<Pool<Postgres>>,
    auth_state: SessionState,
    selected_mailbox: Option<String>,
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
            selected_mailbox: None,
        }
    }

    pub fn send_greeting(&mut self) -> Greeting {
        self.auth_state = SessionState::NotAuthenticated;
        Greeting::default()
    }

    pub fn is_logged_out(&self) -> bool {
        matches!(self.auth_state, SessionState::Logout)
    }

    pub async fn handle_command(&mut self, command: ClientCommand) -> ServerResponse {
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
                ClientCommand::Select(_) => todo!("This should return an error"),
                ClientCommand::Logout(cmd) => self.handle_logout_command(cmd),
            },
            SessionState::Authenticated => match command {
                ClientCommand::Capability(_) => todo!("This should return an error"),
                ClientCommand::StartTLS(_) => todo!("This should return an error"),
                ClientCommand::Login(_) => todo!("This should return an error"),
                ClientCommand::List(cmd) => self.handle_list_command(cmd),
                ClientCommand::Select(cmd) => self.handle_select_command(cmd).await,
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

    async fn handle_select_command(&mut self, cmd: SelectCommand) -> ServerResponse {
        assert!(cmd.mailbox == "INBOX");
        self.selected_mailbox = Some(cmd.mailbox.clone());

        let exists: usize = sqlx::query_scalar!("SELECT COUNT(*) FROM mail")
            .fetch_one(&*self.db_pool)
            .await
            .expect("failed to query mailbox message count")
            .unwrap_or(0) as usize;

        let max_id: i32 = sqlx::query_scalar!("SELECT MAX(uid) FROM mail")
            .fetch_one(&*self.db_pool)
            .await
            .expect("failed to query max mail id")
            .unwrap_or(0);
        let next_uid = max_id as u64 + 1;

        // TODO: real UIDVALIDITY needs per-mailbox metadata; fixed until then.
        let validity_uid = 1;

        SelectResponse::new(cmd, exists, next_uid, validity_uid).into()
    }
}
