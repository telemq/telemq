use mqtt_packets::v_3_1_1::topic::Topic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest<'a> {
    pub broker_id: &'a String,
    pub account_id: &'a String,
    pub cluster_id: &'a String,
    pub socket_addr: &'a String,
    pub client_id: &'a String,
    pub username: &'a Option<String>,
    pub password: &'a Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub connection_allowed: bool,
    pub topics_acl: Option<Vec<TopicACL>>,
    pub max_packet_size: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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
