use crate::get_config::get_config;
use reqwest::StatusCode;

#[derive(serde::Serialize, Debug)]
pub struct DeviceRegisterRequest {
    client_id: String,
    username: String,
    password: String,
}

pub fn device_register(
    client_id: String,
    username: String,
    password: String,
) -> crate::error::ExecResult<String> {
    let server_url = get_config();

    let client = reqwest::blocking::Client::new();
    let request = DeviceRegisterRequest {
        client_id,
        username,
        password,
    };

    let res = client
        .post(format!("{server_url}/devices"))
        .json(&request)
        .send()?;

    match res.status() {
        StatusCode::CREATED => {
            return Ok(format!("Device {} added successfully", request.client_id))
        }
        code => {
            println!("UNIMPLEMENTED STATUS CODE {code}");
            todo!()
        }
    }
}

pub fn device_register_batch(file_path: String) {
    println!("devices from {file_path} auth file registered");
}
