use super::types::DeviceInfo;
use crate::get_config::get_config;
use reqwest::StatusCode;

pub fn device_list() -> crate::error::ExecResult<Vec<DeviceInfo>> {
    let server_add = get_config();

    let res = reqwest::blocking::get(format!("{server_add}/devices"))?;

    match res.status() {
        StatusCode::OK => return Ok(res.json()?),
        code => {
            println!("UNIMPLEMENTED STATUS CODE {code}");
            todo!()
        }
    }
}
