use std::net::IpAddr;

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatusInfo {
    pub id: String,
    pub status: DeviceOnlineStatus,
    pub ip: Option<IpAddr>,
}

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeviceOnlineStatus {
    // #[serde(rename = "online")]
    Online,
    // #[serde(rename = "offline")]
    Offline,
}

#[derive(serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
