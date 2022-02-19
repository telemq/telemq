use std::io::Error as IoError;
use std::net::AddrParseError;

use toml::de::Error as TomlError;

#[derive(Debug)]
pub enum AuthenticatorInitError {
    AuthFile(String),
    Server(String),
}

impl From<IoError> for AuthenticatorInitError {
    fn from(err: IoError) -> Self {
        AuthenticatorInitError::AuthFile(format!("[Authenticator] {:?}", err))
    }
}

impl From<TomlError> for AuthenticatorInitError {
    fn from(err: TomlError) -> Self {
        AuthenticatorInitError::AuthFile(format!("[Authenticator] {:?}", err))
    }
}

impl From<AddrParseError> for AuthenticatorInitError {
    fn from(err: AddrParseError) -> Self {
        AuthenticatorInitError::Server(format!("[Authenticator] {:?}", err))
    }
}

pub type AuthenticatorInitResult<T> = Result<T, AuthenticatorInitError>;

#[derive(Debug)]
pub struct AuthenticatorError;

pub type AuthenticatorResult<R> = Result<R, AuthenticatorError>;
