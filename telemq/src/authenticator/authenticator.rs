use log::info;
use std::net::SocketAddr;

use super::{
    authenticator_error::{AuthenticatorInitResult, AuthenticatorResult},
    authenticator_file::AuthenticatorFile,
    authenticator_server_client::connect as server_connect,
    authenticator_types::{LoginRequest, LoginResponse, TopicACL, TopicAccess},
};
use crate::config::TeleMQServerConfig;

pub use super::authenticator_file::{AccessType, ClientCredentials, ClientRules};

impl From<&AccessType> for TopicAccess {
    fn from(ta: &AccessType) -> TopicAccess {
        match ta {
            &AccessType::Deny => TopicAccess::Deny,
            &AccessType::Read => TopicAccess::Read,
            &AccessType::ReadWrite => TopicAccess::ReadWrite,
            &AccessType::Write => TopicAccess::Write,
        }
    }
}

pub struct Authenticator {
    broker_id: String,
    cluster_id: String,
    account_id: String,
    anonymous_allowed: bool,
    max_packet_size: Option<usize>,
    auth_file: Option<AuthenticatorFile>,
    auth_server: Option<String>,
}

impl Authenticator {
    pub fn new(config: &TeleMQServerConfig) -> AuthenticatorInitResult<Self> {
        info!("[Authenticator]: Initializing with config\n{:?}", config);
        let mut this = Authenticator {
            broker_id: config.broker_id.clone(),
            cluster_id: config.cluster_id.clone(),
            account_id: config.account_id.clone(),
            anonymous_allowed: config.anonymous_allowed,
            max_packet_size: config.max_packet_size.clone(),
            auth_file: None,
            auth_server: config.auth_endpoint.clone(),
        };

        if let Some(ref auth_file_path) = config.auth_file {
            info!("Initializing Authenticator File");
            let file = AuthenticatorFile::new(auth_file_path, config.anonymous_allowed)?;
            this.auth_file = Some(file);
        }

        Ok(this)
    }

    pub async fn connect(
        &self,
        socket_addr: SocketAddr,
        client_id: String,
        username: Option<String>,
        password: Option<String>,
    ) -> AuthenticatorResult<LoginResponse> {
        let connection_allowed = match self.auth_file {
            Some(ref auth_file) => auth_file.login(socket_addr, &client_id, username, password),
            None => match self.auth_server {
                Some(ref addr) => {
                    let req = LoginRequest {
                        broker_id: &self.broker_id,
                        cluster_id: &self.cluster_id,
                        account_id: &self.account_id,
                        socket_addr: &format!("{}", socket_addr),
                        client_id: &client_id,
                        username: &username,
                        password: &password,
                    };
                    return server_connect(addr, req).await;
                }
                None => self.anonymous_allowed,
            },
        };

        if !connection_allowed {
            return Ok(LoginResponse {
                connection_allowed: false,
                topics_acl: None,
                max_packet_size: self.max_packet_size.clone(),
            });
        }

        Ok(LoginResponse {
            connection_allowed: true,
            topics_acl: self.auth_file.as_ref().map(|ref auth_file| {
                let client_rules = match auth_file.get_topics_acl(&client_id) {
                    Some(r) => r,
                    None => {
                        return vec![];
                    }
                };
                client_rules
                    .topic_rules
                    .iter()
                    .map(|r| TopicACL {
                        topic: r.topic.clone(),
                        access: r
                            .access
                            .as_ref()
                            .map(|x| TopicAccess::from(x))
                            .unwrap_or_else(|| TopicAccess::ReadWrite),
                    })
                    .collect()
            }),
            max_packet_size: self.max_packet_size.clone(),
        })
    }

    #[allow(dead_code)]
    pub async fn register_device(
        &mut self,
        credentials: ClientCredentials,
        topic_rules: ClientRules,
    ) {
        if let Some(ref mut auth_file) = self.auth_file {
            auth_file.add_device(credentials, topic_rules);
        }
    }

    #[allow(dead_code)]
    pub async fn deregister_device(&mut self, client_id: String) {
        if let Some(ref mut auth_file) = self.auth_file {
            auth_file.remove_device(client_id);
        }
    }
}