use util::EncodeTo;

use crate::{command::LoginCommand, response::ServerResponseTrait};

#[derive(Debug)]
pub struct LoginResponse {
    pub request_tag: String,
    pub result: LoginResult,
}

#[derive(Debug)]
pub enum LoginResult {
    Ok,
    No,
}

impl ServerResponseTrait<LoginCommand> for LoginResponse {
    fn respond_to(cmd: LoginCommand) -> Self {
        let request_tag = cmd.tag;
        // TODO: Actually implement auth
        let result = LoginResult::Ok;

        Self {
            request_tag,
            result,
        }
    }
}

impl EncodeTo for LoginResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let tag = self.request_tag;

        match self.result {
            LoginResult::Ok => {
                buf.extend_from_slice(format!("{tag} OK LOGIN completed\r\n").as_bytes())
            }
            LoginResult::No => {
                buf.extend_from_slice(format!("{tag} No LOGIN incorrect creds\r\n").as_bytes())
            }
        }
    }
}
