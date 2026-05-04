use std::net::Ipv4Addr;

use slskr_protocol::server::{
    LoginRequest, LoginResponse, ObfuscatedPort, ServerMessage, WaitPort,
};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    stream::ServerConnection,
    version::{CLIENT_MAJOR_VERSION, CLIENT_MINOR_VERSION},
    ClientError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
    pub major_version: u32,
    pub minor_version: u32,
}

impl LoginCredentials {
    #[must_use]
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        major_version: u32,
        minor_version: u32,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            major_version,
            minor_version,
        }
    }

    #[must_use]
    pub fn default_client(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::new(
            username,
            password,
            CLIENT_MAJOR_VERSION,
            CLIENT_MINOR_VERSION,
        )
    }

    #[must_use]
    pub fn login_hash(&self) -> String {
        md5_hex(format!("{}{}", self.username, self.password))
    }

    #[must_use]
    pub fn password_hash(&self) -> String {
        md5_hex(&self.password)
    }

    #[must_use]
    pub fn into_login_request(self) -> LoginRequest {
        let hash = self.login_hash();
        LoginRequest {
            username: self.username,
            password: self.password,
            major_version: self.major_version,
            hash,
            minor_version: self.minor_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub greeting: String,
    pub ip: Ipv4Addr,
    pub password_hash: String,
    pub is_supporter: bool,
}

#[derive(Debug)]
pub struct ServerSession<S> {
    connection: ServerConnection<S>,
}

impl<S> ServerSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    #[must_use]
    pub const fn new(connection: ServerConnection<S>) -> Self {
        Self { connection }
    }

    pub fn into_inner(self) -> ServerConnection<S> {
        self.connection
    }

    pub async fn login(
        &mut self,
        credentials: LoginCredentials,
    ) -> Result<SessionInfo, ClientError> {
        self.connection
            .send(&ServerMessage::LoginRequest(
                credentials.into_login_request(),
            ))
            .await?;

        match self.connection.receive().await? {
            ServerMessage::LoginResponse(LoginResponse::Success {
                greet,
                ip,
                hash,
                is_supporter,
            }) => Ok(SessionInfo {
                greeting: greet,
                ip,
                password_hash: hash,
                is_supporter,
            }),
            ServerMessage::LoginResponse(LoginResponse::Failure { reason, detail }) => {
                Err(ClientError::LoginRejected {
                    reason,
                    detail: detail
                        .map(|value| format!(" ({value})"))
                        .unwrap_or_default(),
                })
            }
            message => Err(ClientError::unexpected_server_message(message)),
        }
    }

    pub async fn set_wait_port(&mut self, port: u32) -> Result<(), ClientError> {
        self.connection
            .send(&ServerMessage::SetWaitPort(WaitPort {
                port,
                obfuscation: None,
            }))
            .await
    }

    pub async fn set_wait_port_obfuscated(
        &mut self,
        port: u32,
        obfuscation_kind: u32,
        obfuscated_port: u32,
    ) -> Result<(), ClientError> {
        self.connection
            .send(&ServerMessage::SetWaitPort(WaitPort {
                port,
                obfuscation: Some(ObfuscatedPort {
                    kind: obfuscation_kind,
                    port: obfuscated_port,
                }),
            }))
            .await
    }

    pub async fn send_ping(&mut self) -> Result<(), ClientError> {
        self.connection.send(&ServerMessage::ServerPing).await
    }

    pub async fn send_server_message(&mut self, message: ServerMessage) -> Result<(), ClientError> {
        self.connection.send(&message).await
    }

    pub async fn receive(&mut self) -> Result<ServerMessage, ClientError> {
        self.connection.receive().await
    }
}

fn md5_hex(value: impl AsRef<[u8]>) -> String {
    format!("{:x}", md5::compute(value))
}
