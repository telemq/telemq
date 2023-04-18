use std::{fs::read as read_file, net::SocketAddr, path::Path};

use crypto::{digest::Digest, sha2::Sha256};
use ipnet::IpNet;
use log::error;
use mqtt_packets::v_3_1_1::topic::Topic;
use serde::{Deserialize, Serialize};
use toml::from_slice;

use super::authenticator_error::*;

#[derive(Debug)]
pub struct AuthenticatorFile {
    anonymous_allowed: bool,
    #[allow(unused)]
    topic_all_rules: Option<Vec<TopicRule>>,
    topic_client_rules: Option<Vec<ClientRules>>,
    credentials: Option<Vec<ClientCredentials>>,
    ip_whitelist: Option<Vec<IpNet>>,
    ip_blacklist: Option<Vec<IpNet>>,
}

impl AuthenticatorFile {
    const CLIENT_ID_PATTERN: &'static str = "{client_id}";

    pub fn new<P: AsRef<Path>>(file: P, anonymous_allowed: bool) -> AuthenticatorInitResult<Self> {
        let src = AuthenticatorFileSrc::try_from_file(file)?;
        Ok(AuthenticatorFile {
            anonymous_allowed,
            topic_all_rules: match src.topic_all_rules {
                Some(all_rules) => {
                    let mut r = Vec::with_capacity(all_rules.len());
                    for rule in all_rules {
                        r.push(TopicRule {
                            access: rule.access,
                            topic: Topic::make_from_string(&rule.topic),
                        });
                    }
                    Some(r)
                }
                None => None,
            },
            topic_client_rules: match src.topic_client_rules {
                Some(client_rules) => {
                    let mut c = Vec::with_capacity(client_rules.len());

                    for client in client_rules {
                        let mut topic_rules = Vec::with_capacity(client.topic_rules.len());

                        for rule in client.topic_rules {
                            topic_rules.push(TopicRule {
                                access: rule.access,
                                topic: Topic::make_from_string(
                                    &rule
                                        .topic
                                        .replace(Self::CLIENT_ID_PATTERN, &client.client_id),
                                ),
                            });
                        }
                        c.push(ClientRules {
                            client_id: client.client_id,
                            topic_rules,
                        });
                    }
                    Some(c)
                }
                None => None,
            },
            credentials: src.credentials,
            ip_whitelist: src
                .ip_whitelist
                .map(|v| v.iter().map(|s| s.parse().unwrap()).collect()),
            ip_blacklist: src
                .ip_blacklist
                .map(|v| v.iter().map(|s| s.parse().unwrap()).collect()),
        })
    }

    #[allow(dead_code)]
    pub fn add_device(&mut self, mut credentials: ClientCredentials, client_topics: ClientRules) {
        if let Some(ref mut all_clients_topic_rules) = self.topic_client_rules {
            all_clients_topic_rules.retain(|t| t.client_id != credentials.client_id);
            all_clients_topic_rules.push(client_topics);
        } else {
            let mut all_clients_topic_rules = vec![];
            all_clients_topic_rules.push(client_topics);
            self.topic_client_rules = Some(all_clients_topic_rules);
        }

        if let Some(ref mut all_credentials) = self.credentials {
            all_credentials.retain(|c| c.client_id != credentials.client_id);
            credentials.password = Self::get_hash_password(&credentials.password);
            all_credentials.push(credentials);
        } else {
            let mut all_credentials = vec![];
            credentials.password = Self::get_hash_password(&credentials.password);
            all_credentials.push(credentials);
            self.credentials = Some(all_credentials);
        }
    }

    #[allow(dead_code)]
    pub fn remove_device(&mut self, client_id: String) {
        if let Some(ref mut all_clients_topic_rules) = self.topic_client_rules {
            all_clients_topic_rules.retain(|t| t.client_id != client_id);
        }

        if let Some(ref mut all_credentials) = self.credentials {
            all_credentials.retain(|c| c.client_id != client_id);
        }
    }

    // false - not authorized to log in
    // true - authorized to log in
    pub fn login(
        &self,
        socket_addr: SocketAddr,
        client_id: &String,
        maybe_username: Option<String>,
        maybe_password: Option<String>,
    ) -> bool {
        let ip_net_addr = IpNet::from(socket_addr.ip());
        let blacklisted = self
            .ip_blacklist
            .as_ref()
            .map(|blacklisted_sockets| {
                blacklisted_sockets
                    .iter()
                    .any(|black_net| black_net.contains(&ip_net_addr))
            })
            .unwrap_or(false);

        if blacklisted {
            error!(
                "[Authenticator File] IP blacklisted. Client ID {}, IP {:?}",
                client_id, socket_addr
            );
            return false;
        }

        let whitelisted = match &self.ip_whitelist {
            &Some(ref whitelisted_ips) => whitelisted_ips
                .iter()
                .any(|white_net| white_net.contains(&ip_net_addr)),
            // whitelist by default if ip_whitelist config property is not defined
            &None => true,
        };

        if !whitelisted {
            error!(
                "IP is not whitelisted. Client ID {}, IP {:?}",
                client_id, socket_addr
            );
            return false;
        }

        match self.credentials {
            Some(ref credentials_list) => {
                let (username, password) = match (maybe_username, maybe_password) {
                    (Some(u), Some(p)) => (u, p),
                    _ => {
                        // credentials list is provided in the file,
                        // which means no anonymous clients are allowed and
                        // username and password should be provided
                        return false;
                    }
                };
                let password_hash = Self::get_hash_password(password.as_str());
                return credentials_list
                    .iter()
                    .find(|credentials_entry| {
                        &credentials_entry.client_id == client_id
                            && credentials_entry.username == username
                            && password_hash == credentials_entry.password
                    })
                    .is_some();
            }
            None => return self.anonymous_allowed,
        }
    }

    pub fn get_topics_acl(&self, client_id: &String) -> Option<&ClientRules> {
        match self.topic_client_rules {
            Some(ref clients_rules) => clients_rules.iter().find(|cr| &cr.client_id == client_id),
            None => None,
        }
    }

    fn get_hash_password(raw_password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.input_str(raw_password);
        hasher.result_str()
    }
}

#[derive(Debug, Deserialize)]
pub struct AuthenticatorFileSrc {
    topic_all_rules: Option<Vec<TopicRuleSrc>>,
    topic_client_rules: Option<Vec<ClientRulesSrc>>,
    credentials: Option<Vec<ClientCredentials>>,
    ip_whitelist: Option<Vec<String>>,
    ip_blacklist: Option<Vec<String>>,
}

impl AuthenticatorFileSrc {
    pub fn try_from_file<P: AsRef<Path>>(path: P) -> AuthenticatorInitResult<Self> {
        let authenticator_file_content = read_file(path)?;
        let authenticator_file: AuthenticatorFileSrc =
            from_slice(authenticator_file_content.as_slice())?;
        Ok(authenticator_file)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TopicRuleSrc {
    pub access: Option<AccessType>,
    pub topic: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum AccessType {
    Read,
    Write,
    ReadWrite,
    Deny,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientRulesSrc {
    pub client_id: String,
    pub topic_rules: Vec<TopicRuleSrc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientCredentials {
    client_id: String,
    username: String,
    password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TopicRule {
    pub access: Option<AccessType>,
    pub topic: Topic,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientRules {
    pub client_id: String,
    pub topic_rules: Vec<TopicRule>,
}
