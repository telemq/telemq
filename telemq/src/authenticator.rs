use log::info;
use mqtt_packets::v_3_1_1::topic::Topic;
use serde::Deserialize;
use std::net::SocketAddr;

use crate::{
    authenticator_error::AuthenticatorInitResult, authenticator_file::AuthenticatorFile,
    config::TeleMQServerConfig,
};

pub use crate::authenticator_file::{AccessType, ClientCredentials, ClientRules};

#[derive(Debug, Deserialize)]
pub struct TopicACL {
    pub topic: Topic,
    pub access: TopicAccess,
}

#[derive(Debug, Deserialize)]
pub enum TopicAccess {
    Read,
    Write,
    ReadWrite,
    Deny,
}

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

#[derive(Debug)]
pub struct ConnectResponse {
    pub connection_allowed: bool,
    pub topics_acl: Vec<TopicACL>,
    pub max_packet_size: Option<usize>,
}

pub struct Authenticator {
    anonymous_allowed: bool,
    max_packet_size: Option<usize>,
    auth_file: Option<AuthenticatorFile>,
    _auth_server: Option<SocketAddr>,
}

impl Authenticator {
    pub fn new(config: &TeleMQServerConfig) -> AuthenticatorInitResult<Self> {
        info!("[Authenticator]: Initializing with config\n{:?}", config);
        let mut this = Authenticator {
            anonymous_allowed: config.anonymous_allowed,
            max_packet_size: config.max_packet_size.clone(),
            auth_file: None,
            _auth_server: config.auth_endpoint,
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
    ) -> AuthenticatorResult<ConnectResponse> {
        let connection_allowed = match self.auth_file {
            Some(ref auth_file) => auth_file.login(socket_addr, &client_id, username, password),
            None => username.is_none() && password.is_none() && self.anonymous_allowed,
        };

        if !connection_allowed {
            return Ok(ConnectResponse {
                connection_allowed: false,
                topics_acl: vec![],
                max_packet_size: self.max_packet_size.clone(),
            });
        }

        Ok(ConnectResponse {
            connection_allowed: true,
            topics_acl: self
                .auth_file
                .as_ref()
                .map(|ref auth_file| {
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
                })
                .unwrap_or_else(|| vec![]),
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

#[derive(Debug)]
pub struct AuthenticatorError;

pub type AuthenticatorResult<R> = Result<R, AuthenticatorError>;
