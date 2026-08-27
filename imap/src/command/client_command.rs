use crate::command::{CapabilityCommand, StartTLSCommand};

#[derive(Debug)]
pub enum ClientCommand {
    Capability(CapabilityCommand),
    StartTLS(StartTLSCommand),
}

impl ClientCommand {
    pub fn parse_bytes(buf: &[u8]) -> Option<(Self, usize)> {
        let str = str::from_utf8(buf).unwrap();
        dbg!(str);
        let lines = str.split("\r\n").collect::<Vec<_>>();
        let line = lines[0];
        println!("Got line: {:?}", line);

        let mut split = line.splitn(3, " ");
        let tag = split.next().unwrap().to_string();
        let command_text = split.next().unwrap();
        let rest = split
            .next()
            .unwrap_or_default()
            .split(" ")
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let bytes_read = line.len() + 2; // +2 for the \r\n. TODO: This might not always be the case

        dbg!(&command_text);

        match command_text {
            "CAPABILITY" => Some((CapabilityCommand::with_args(tag, &rest).into(), bytes_read)),
            "STARTTLS" => Some((StartTLSCommand::with_args(tag, &rest).into(), bytes_read)),
            _ => None,
        }
    }
}

pub trait ClientCommandTrait {
    fn with_args(tag: String, args: &[String]) -> Self;
}
