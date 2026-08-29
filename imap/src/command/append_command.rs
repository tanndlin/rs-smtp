use crate::{
    client_command_from_impl,
    command::{ClientCommand, client_command::ClientCommandTrait},
};

#[derive(Debug)]
pub struct AppendCommand {
    pub tag: String,
    pub mailbox: String,
    pub flags: Vec<String>,
    pub date_time: Option<String>,
    pub message: Vec<u8>,
}

impl ClientCommandTrait for AppendCommand {
    fn with_args(tag: String, args: &[String]) -> Self {
        let mailbox = args[0].clone();

        let length: usize = args
            .last()
            .unwrap()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .parse()
            .unwrap(); // TODO: Handle the +

        Self {
            tag,
            mailbox,
            flags: vec![],
            date_time: None,
            message: vec![],
        }
    }
}

client_command_from_impl!(AppendCommand, Append);
