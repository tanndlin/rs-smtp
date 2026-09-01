use std::sync::Arc;

use sqlx::{Pool, Postgres};

use crate::{
    command::{
        AppendCommand, ClientCommand, ClientCommandTrait, FetchCommand, ListCommand, LogoutCommand,
        SelectCommand, StatusCommand,
    },
    response::{
        AppendOkResponse, CapabilityResponse, ContinuationResponse, Greeting, ListResponse,
        LoginResponse, LoginResult, LogoutResponse, MailboxListEntry, SelectResponse,
        ServerErrorReason, ServerErrorResponse, ServerResponse, ServerResponseTrait,
        StatusResponse,
    },
};

pub struct IMAPSession {
    db_pool: Arc<Pool<Postgres>>,
    auth_state: SessionState,
    selected_mailbox: Option<String>,
    expecting_append_mail: Option<AppendCommand>, // TODO: This doesnt support pipelining, but i haven't looked into how that actually works anyways
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
            expecting_append_mail: None,
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
                ClientCommand::Logout(cmd) => self.handle_logout_command(cmd),
                ClientCommand::List(cmd) => {
                    cmd.protocol_violation("Not authorized".to_string()).into()
                }
                ClientCommand::Select(cmd) => {
                    cmd.protocol_violation("Not authorized".to_string()).into()
                }
                ClientCommand::Status(cmd) => {
                    cmd.protocol_violation("Not authorized".to_string()).into()
                }
                ClientCommand::Fetch(cmd) => {
                    cmd.protocol_violation("Not authorized".to_string()).into()
                }
                ClientCommand::Append(cmd) => {
                    cmd.protocol_violation("Not authorized".to_string()).into()
                }
            },
            SessionState::Authenticated => match command {
                ClientCommand::List(cmd) => self.handle_list_command(cmd),
                ClientCommand::Select(cmd) => self.handle_select_command(cmd).await,
                ClientCommand::Status(cmd) => self.handle_status_command(cmd).await,
                ClientCommand::Fetch(cmd) => self.handle_fetch_command(cmd).await,
                ClientCommand::Append(cmd) => self.handle_append_command(cmd).await,
                ClientCommand::Logout(cmd) => self.handle_logout_command(cmd),
                ClientCommand::Capability(_)
                | ClientCommand::StartTLS(_)
                | ClientCommand::Login(_) => todo!("This should return an error"),
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

        let validity_uid = sqlx::query_scalar!(
            "SELECT uid_validity FROM mailboxes WHERE name = $1",
            self.selected_mailbox
        )
        .fetch_one(&*self.db_pool)
        .await
        .unwrap() as u64;

        SelectResponse::new(cmd, exists, next_uid, validity_uid).into()
    }

    async fn handle_status_command(&mut self, cmd: StatusCommand) -> ServerResponse {
        assert!(cmd.mailbox == "INBOX");

        let messages = if cmd.messages {
            Some(
                sqlx::query_scalar!("SELECT COUNT(*) FROM mail")
                    .fetch_one(&*self.db_pool)
                    .await
                    .expect("failed to query mailbox message count")
                    .unwrap_or(0) as u64,
            )
        } else {
            None
        };

        let next_uid = if cmd.next_uid {
            let max_id = sqlx::query_scalar!("SELECT MAX(uid) FROM mail")
                .fetch_one(&*self.db_pool)
                .await
                .expect("failed to query max mail id")
                .unwrap_or(0) as u64;
            Some(max_id + 1)
        } else {
            None
        };

        let validity_uid = if cmd.validity_uid {
            Some(
                sqlx::query_scalar!(
                    "SELECT uid_validity FROM mailboxes WHERE name = $1",
                    self.selected_mailbox
                )
                .fetch_one(&*self.db_pool)
                .await
                .unwrap() as u64,
            )
        } else {
            None
        };

        //TODO: After flags are added, this needs to be counted
        let unseen = if cmd.unseen {
            Some(
                sqlx::query_scalar!("SELECT COUNT(*) FROM mail")
                    .fetch_one(&*self.db_pool)
                    .await
                    .expect("failed to query mailbox message count")
                    .unwrap_or(0) as u64,
            )
        } else {
            None
        };

        //TODO: After flags are added, this needs to be counted
        let deleted = if cmd.deleted { Some(0) } else { None };

        StatusResponse::new(cmd, messages, next_uid, validity_uid, unseen, deleted).into()
    }

    async fn handle_fetch_command(&self, cmd: FetchCommand) -> ServerResponse {
        todo!()
    }

    async fn handle_append_command(&self, cmd: AppendCommand) -> ServerResponse {
        // Check if the mailbox exists
        let mailbox = sqlx::query!("SELECT name FROM mailboxes WHERE name = $1", cmd.mailbox)
            .fetch_optional(&*self.db_pool)
            .await
            .expect("Failed to look up mailbox");
        if mailbox.is_none() {
            return ServerResponse::Error(ServerErrorResponse {
                tag: Some(cmd.tag.to_string()),
                reason: ServerErrorReason::Deny("TRYCREATE".to_string()),
            });
        }

        match cmd.message.take() {
            Some(message) => {
                let (uidvalidity, uid) = self.store_appended_message(&cmd, &message).await;
                AppendOkResponse::new(cmd.tag, uidvalidity, uid).into()
            }
            None => {
                // synchronizing literal: tell the client to send the bytes,
                // and remember we're mid-APPEND so the next read isn't parsed as a command
                self.expecting_append_mail = Some(cmd);
                ContinuationResponse { tag: cmd.tag }.into() // encodes `+ ...\r\n`
            }
        }
    }
}
