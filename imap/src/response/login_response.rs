use util::EncodeTo;

use crate::{
    command::LoginCommand,
    response::{ServerResponse, ServerResponseTrait},
    server_response_from_impl,
};

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

server_response_from_impl!(LoginResponse, Login);

impl EncodeTo for LoginResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let tag = self.request_tag;

        match self.result {
            LoginResult::Ok => buf.extend(format!("{tag} OK LOGIN completed\r\n").bytes()),
            LoginResult::No => buf.extend(format!("{tag} No LOGIN incorrect creds\r\n").bytes()),
        }
    }
}
