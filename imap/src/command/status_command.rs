use crate::{
    client_command_from_impl,
    command::client_command::{ClientCommand, ClientCommandTrait},
};

#[derive(Debug)]
pub struct StatusCommand {
    pub tag: String,
    pub mailbox: String,

    // Whether these were asked for
    pub messages: bool,
    pub next_uid: bool,
    pub validity_uid: bool,
    pub unseen: bool,
    pub deleted: bool,
    pub size: bool,
}

impl ClientCommandTrait for StatusCommand {
    fn with_args(tag: String, args: &[String]) -> Self {
        dbg!(args);
        assert!(args.len() > 0);
        let mailbox = args[0].trim_matches('"').to_string();
        let flags = args[1..]
            .iter()
            .map(|f| f.trim_start_matches('(').trim_end_matches(')').to_string())
            .collect::<Vec<_>>(); // Could be hash set for O(1) lookup but there is a limit set of flags

        let messages = flags.contains(&"MESSAGES".to_string());
        let next_uid = flags.contains(&"UIDNEXT".to_string());
        let validity_uid = flags.contains(&"UIDVALIDITY".to_string());
        let unseen = flags.contains(&"UNSEEN".to_string());
        let deleted = flags.contains(&"DELETED".to_string());
        let size = flags.contains(&"SIZE".to_string());

        Self {
            tag,
            mailbox,
            messages,
            next_uid,
            validity_uid,
            unseen,
            deleted,
            size,
        }
    }
}

client_command_from_impl!(StatusCommand, Status);
