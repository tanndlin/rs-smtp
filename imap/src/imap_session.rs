use std::{collections::HashMap, sync::Arc};

use sqlx::{Pool, Postgres, types::chrono::Utc};

use crate::{
    command::{
        AppendCommand, BodyFetchable, ClientCommand, ClientCommandTrait, CreateCommand,
        FetchCommand, Fetchable, ListCommand, LogoutCommand, LsubCommand, SelectCommand, Sequence,
        StatusCommand, StoreCommand, UIDCommand, UIDCommandType,
    },
    cursor::Cursor,
    handle_fetch::get_fetchable,
    response::{
        AppendOkResponse, CapabilityResponse, ContinuationResponse, CreateResponse,
        FetchMessageResponse, FetchResponse, Greeting, ListResponse, LoginResponse, LoginResult,
        LogoutResponse, LsubResponse, MailboxListEntry, NoopResponse, SelectResponse,
        ServerErrorReason, ServerErrorResponse, ServerResponse, ServerResponseTrait,
        StatusResponse,
    },
};
use util::Email;

pub struct IMAPSession {
    db_pool: Arc<Pool<Postgres>>,
    auth_state: SessionState,
    selected_mailbox: Option<SelectedMailbox>,
    expecting_append_mail: Option<AppendCommand>, // TODO: This doesnt support pipelining, but i haven't looked into how that actually works anyways
}

pub struct SelectedMailbox {
    id: i32,
    name: String,
}

#[derive(Clone, Copy)]
enum Addressing {
    BySeq,
    ByUid,
}

struct MessageRef {
    id: i32,     // primary key to read the message by
    seqnum: u64, // position in the mailbox to report it under
    uid: i32,
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

    pub async fn handle_bytes(
        &mut self,
        buf: &[u8],
    ) -> Result<(ServerResponse, usize), ServerErrorResponse> {
        if let Some(mut append_cmd) = self.expecting_append_mail.take() {
            // Parse the rest of the message as the mail
            let mut cursor = Cursor::new(buf);
            let length = append_cmd.message_length;
            // Grab the next {length} bytes
            append_cmd.message = Some(cursor.raw(length).to_vec());
            let res = self.handle_command(append_cmd.into()).await;
            self.expecting_append_mail = None;
            cursor.expect_crlf()?;
            return Ok((res, length + 2));
        }

        match ClientCommand::parse_bytes(buf) {
            Ok((command, read)) => {
                dbg!(&command);
                let res = self.handle_command(command).await;
                dbg!(&res);
                Ok((res, read))
            }
            Err(e) => Err(e.into()),
        }
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
                ClientCommand::Noop(noop_command) => NoopResponse::respond_to(noop_command).into(),
                ClientCommand::List(cmd) => {
                    cmd.protocol_violation("Not authorized".to_string()).into()
                }
                ClientCommand::Lsub(cmd) => {
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
                ClientCommand::UID(cmd) => {
                    cmd.protocol_violation("Not authorized".to_string()).into()
                }
                ClientCommand::Create(cmd) => {
                    cmd.protocol_violation("Not authorized".to_string()).into()
                }
                ClientCommand::Store(cmd) => {
                    cmd.protocol_violation("Not authorized".to_string()).into()
                }
            },
            SessionState::Authenticated => match command {
                ClientCommand::List(cmd) => self.handle_list_command(cmd),
                ClientCommand::Lsub(cmd) => self.handle_lsub_command(cmd),
                ClientCommand::Select(cmd) => self.handle_select_command(cmd).await,
                ClientCommand::Status(cmd) => self.handle_status_command(cmd).await,
                ClientCommand::Fetch(cmd) => {
                    self.handle_fetch_command(cmd, Addressing::BySeq).await
                }
                ClientCommand::Append(cmd) => self.handle_append_command(cmd).await,
                ClientCommand::Logout(cmd) => self.handle_logout_command(cmd),
                ClientCommand::Capability(cmd) => CapabilityResponse::respond_to(cmd).into(),
                ClientCommand::UID(cmd) => self.handle_uid(cmd).await,
                ClientCommand::Create(cmd) => self.handle_create_command(cmd).await,
                ClientCommand::Store(cmd) => {
                    self.handle_store_command(cmd, Addressing::BySeq).await
                }
                ClientCommand::Noop(noop_command) => NoopResponse::respond_to(noop_command).into(),
                ClientCommand::StartTLS(_) | ClientCommand::Login(_) => {
                    todo!("This should return an error")
                }
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
        let mailboxes: Vec<MailboxListEntry> = vec![inbox];
        ListResponse::new(cmd.tag, mailboxes).into()
    }

    fn handle_lsub_command(&mut self, cmd: LsubCommand) -> ServerResponse {
        // TODO
        let inbox = MailboxListEntry::new(
            vec!["\\Unmarked", "\\HasNoChildren"],
            None,
            "Inbox".to_string(),
        );
        let mailboxes: Vec<MailboxListEntry> = vec![inbox];
        LsubResponse::new(cmd.tag, mailboxes).into()
    }

    async fn handle_select_command(&mut self, cmd: SelectCommand) -> ServerResponse {
        assert!(cmd.mailbox == "INBOX");
        let mailbox_id =
            sqlx::query_scalar!("SELECT id FROM mailboxes WHERE name = $1", &cmd.mailbox)
                .fetch_one(&*self.db_pool)
                .await
                .expect("Failed to fetch mailbox for id");
        self.selected_mailbox = Some(SelectedMailbox {
            id: mailbox_id,
            name: cmd.mailbox.clone(),
        });

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
            self.selected_mailbox.as_ref().unwrap().name
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

        // TODO: Repeated SQL that can get merged
        let validity_uid = if cmd.validity_uid {
            Some(
                sqlx::query_scalar!(
                    "SELECT uid_validity FROM mailboxes WHERE name = $1",
                    self.selected_mailbox.as_ref().unwrap().name
                )
                .fetch_one(&*self.db_pool)
                .await
                .unwrap() as u64,
            )
        } else {
            None
        };

        let unseen = if cmd.unseen {
            Some(
                sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM mail WHERE NOT ($1 = ANY(flags))",
                    "\\Seen"
                )
                .fetch_one(&*self.db_pool)
                .await
                .expect("failed to query unseen count")
                .unwrap_or(0) as u64,
            )
        } else {
            None
        };

        //TODO: Flags lookups can get merged
        let deleted = if cmd.deleted {
            Some(
                sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM mail WHERE $1 = ANY(flags)",
                    "\\Deleted"
                )
                .fetch_one(&*self.db_pool)
                .await
                .expect("failed to query deleted count")
                .unwrap_or(0) as u64,
            )
        } else {
            None
        };

        StatusResponse::new(cmd, messages, next_uid, validity_uid, unseen, deleted).into()
    }

    /// Resolve a sequence set against the messages that actually exist.
    /// A number matching nothing is skipped
    async fn resolve_sequence_set(
        &self,
        sequences: &[Sequence],
        addressing: Addressing,
    ) -> Vec<MessageRef> {
        // A sequence number is a message's position in the mailbox ordered by
        // UID, so one pass serves both addressing modes.
        // TODO: Should be scoped to the selected mailbox once INBOX isn't the only one
        let rows = sqlx::query!("SELECT id, uid FROM mail ORDER BY uid")
            .fetch_all(&*self.db_pool)
            .await
            .expect("failed to list mailbox messages");

        // `*` is the last message for a sequence number set, but the largest
        // UID for a UID set - the two only coincide in a mailbox that has never
        // been expunged.
        let last = match addressing {
            Addressing::BySeq => rows.len() as u64,
            Addressing::ByUid => rows.last().map_or(0, |row| row.uid as u64),
        };

        rows.into_iter()
            .enumerate()
            .filter_map(|(i, row)| {
                let seqnum = i as u64 + 1;
                let n = match addressing {
                    Addressing::BySeq => seqnum,
                    Addressing::ByUid => row.uid as u64,
                };

                Sequence::set_contains(sequences, n, last)
                    .then_some(MessageRef {
                        id: row.id,
                        seqnum,
                        uid: row.uid,
                    })
            })
            .collect()
    }

    async fn handle_fetch_command(
        &self,
        cmd: FetchCommand,
        addressing: Addressing,
    ) -> ServerResponse {
        let messages = self.resolve_sequence_set(&cmd.sequences, addressing).await;

        let mut responses = Vec::new();
        for message in messages {
            let mut metadata: HashMap<String, String> = HashMap::new();
            for fetchable in &cmd.fetch_list {
                let expanded: &[Fetchable] = match fetchable {
                    Fetchable::Fast => &[
                        Fetchable::Flags,
                        Fetchable::Internaldate,
                        Fetchable::RFC822Size,
                    ],
                    Fetchable::All => &[
                        Fetchable::Flags,
                        Fetchable::Internaldate,
                        Fetchable::RFC822Size,
                        Fetchable::Envelope,
                    ],
                    Fetchable::Full => &[
                        Fetchable::Flags,
                        Fetchable::Internaldate,
                        Fetchable::RFC822Size,
                        Fetchable::Envelope,
                        Fetchable::Body(BodyFetchable::Full),
                    ],
                    other => std::slice::from_ref(other),
                };

                for fetchable in expanded {
                    let value = get_fetchable(self.db_pool.clone(), message.id, fetchable).await;
                    metadata.insert(fetchable.to_string(), value);
                }
            }

            // Untagged FETCH responses are keyed by sequence number even when
            // the client addressed the message by UID.
            responses.push(FetchMessageResponse::new(message.seqnum, metadata));
        }

        FetchResponse::new(cmd.tag, responses).into()
    }

    async fn handle_append_command(&mut self, mut cmd: AppendCommand) -> ServerResponse {
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
                let (uid_validity, uid) = self.store_appended_message(&cmd, &message).await;
                AppendOkResponse::new(cmd.tag, uid_validity, uid).into()
            }
            None => {
                // synchronizing literal: tell the client to send the bytes,
                // and remember we're mid-APPEND so the next read isn't parsed as a command
                let tag = cmd.tag.clone();
                self.expecting_append_mail = Some(cmd);
                ContinuationResponse { tag }.into() // encodes `+ ...\r\n`
            }
        }
    }

    /// Persist an appended message and allocate it a UID in `cmd.mailbox`.
    /// Returns `(uid_validity, uid)` for the `APPENDUID` response code.
    async fn store_appended_message(&self, cmd: &AppendCommand, message: &[u8]) -> (u32, u32) {
        let mut email = Email::from_raw(
            cmd.date_time.unwrap_or_else(Utc::now),
            String::from_utf8_lossy(message).into_owned(),
        );
        email.flags = cmd.flags.clone();

        let mut tx = self
            .db_pool
            .begin()
            .await
            .expect("failed to open APPEND transaction");

        let allocated = sqlx::query!(
            "UPDATE mailboxes SET uid_next = uid_next + 1 WHERE name = $1 \
             RETURNING (uid_next - 1) AS \"uid!\", uid_validity",
            cmd.mailbox,
        )
        .fetch_one(&mut *tx)
        .await
        .expect("failed to allocate APPEND uid");

        email
            .insert(&mut *tx, &cmd.mailbox, allocated.uid)
            .await
            .expect("failed to insert appended message");

        tx.commit()
            .await
            .expect("failed to commit appended message");

        (allocated.uid_validity as u32, allocated.uid as u32)
    }

    async fn handle_uid(&self, cmd: UIDCommand) -> ServerResponse {
        match cmd.command {
            UIDCommandType::Fetch {
                sequences,
                mut fetchable,
            } => {
                // UID has to be reported whether or not it was asked for.
                if !fetchable.contains(&Fetchable::UID) {
                    fetchable.push(Fetchable::UID);
                }

                let fetch_cmd = FetchCommand {
                    tag: cmd.tag,
                    sequences,
                    fetch_list: fetchable,
                };
                self.handle_fetch_command(fetch_cmd, Addressing::ByUid)
                    .await
            }
            UIDCommandType::Store { sequences, store } => {
                let store_cmd = StoreCommand {
                    tag: cmd.tag,
                    sequences,
                    store,
                };
                self.handle_store_command(store_cmd, Addressing::ByUid)
                    .await
            }
            UIDCommandType::Copy { sequences, mailbox } => {
                todo!();
            }
        }
    }

    async fn handle_create_command(&self, cmd: CreateCommand) -> ServerResponse {
        // Check if the mailbox exists
        let mailbox = sqlx::query!("SELECT name FROM mailboxes WHERE name = $1", cmd.mailbox)
            .fetch_optional(&*self.db_pool)
            .await
            .expect("Failed to look up mailbox");
        if mailbox.is_some() {
            return ServerResponse::Error(ServerErrorResponse {
                tag: Some(cmd.tag.to_string()),
                reason: ServerErrorReason::Deny("ALREADYEXISTS".to_string()),
            });
        }

        let uid_validity = sqlx::query_scalar!(
            "INSERT INTO mailboxes (name, uid_validity, uid_next) VALUES ($1, 1, 1) RETURNING uid_validity",
            cmd.mailbox
        )
        .fetch_one(&*self.db_pool)
        .await
        .expect("Failed to create mailbox");

        CreateResponse::new(cmd.tag, uid_validity as u32).into()
    }

    async fn handle_store_command(
        &self,
        store_cmd: StoreCommand,
        by_uid: Addressing,
    ) -> ServerResponse {
        let messages = self
            .resolve_sequence_set(&store_cmd.sequences, by_uid)
            .await;

        let mut responses = Vec::new();
        for message in messages {
            let mut metadata: HashMap<String, String> = HashMap::new();
            let flags = store_cmd
                .store
                .apply_to_message(self.db_pool.clone(), message.id)
                .await;

            // `.SILENT` still applies the change, it just doesn't report it.
            if store_cmd.store.silent {
                continue;
            }

            metadata.insert(Fetchable::Flags.to_string(), flags);
            // As with UID FETCH, the client has no sequence number to key a
            // UID STORE response on, so the UID always comes back.
            if let Addressing::ByUid = by_uid {
                metadata.insert(Fetchable::UID.to_string(), message.uid.to_string());
            }
            responses.push(FetchMessageResponse::new(message.seqnum, metadata));
        }

        FetchResponse::new(store_cmd.tag, responses).into()
    }
}
