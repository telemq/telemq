use log::error;
use reqwest::Client;

use super::{
    authenticator_error::AuthenticatorResult,
    authenticator_types::{LoginRequest, LoginResponse},
};

pub async fn connect<'a>(
    url: &String,
    req: LoginRequest<'a>,
) -> AuthenticatorResult<LoginResponse> {
    match Client::new().post(url.clone()).json(&req).send().await {
        Ok(res) => res.json().await.or_else(|_| {
            Ok(LoginResponse {
                connection_allowed: false,
                max_packet_size: None,
                topics_acl: None,
            })
        }),
        Err(err) => {
            error!(
                "[Authenticator Worker]: Authentication Endpoint Error. {:?}",
                err
            );
            Ok(LoginResponse {
                connection_allowed: false,
                max_packet_size: None,
                topics_acl: None,
            })
        }
    }
}
