use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Debug)]
pub struct Device {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceInfo {
    pub id: String,
    pub status: String,
    pub ip: Option<IpAddr>,
}
