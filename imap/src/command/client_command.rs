use crate::command::CapabilityCommand;

#[derive(Debug)]
pub enum ClientCommand {
    Capability(CapabilityCommand),
}

impl ClientCommand {
    pub fn parse_bytes(buf: &[u8]) -> Option<(Self, usize)> {
        let str = str::from_utf8(buf).unwrap();
        let lines = str.split("\r\n").collect::<Vec<_>>();
        let line = lines[0];
        dbg!(&line);

        let mut split = line.splitn(3, " ");
        let id = split.next().unwrap();
        let command_text = split.next().unwrap();
        let rest = split.next().unwrap_or_default().to_string();

        let bytes_read = line.len();

        match command_text {
            "CAPABILITY" => Some((CapabilityCommand::new(rest).into(), bytes_read)),
            _ => None,
        }
    }
}

impl From<&[u8]> for ClientCommand {
    fn from(buf: &[u8]) -> Self {
        todo!()
    }
}

pub trait ClientCommandTrait: Sized {
    fn parse_bytes(buf: &[u8]) -> Option<(Self, usize)>;
}
