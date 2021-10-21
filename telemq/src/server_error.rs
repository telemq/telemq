use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
    io,
};

#[derive(Debug)]
pub struct ServerError(pub String);

impl ServerError {
    // const CONNECTION_POOL_EXHAUSTED: &'static str = "Connections number is exhausted";

    // pub fn connection_pool_exhausted() -> Self {
    //     ServerError(Self::CONNECTION_POOL_EXHAUSTED.into())
    // }
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl StdError for ServerError {}

impl From<io::Error> for ServerError {
    fn from(err: io::Error) -> Self {
        ServerError(err.to_string())
    }
}

impl From<String> for ServerError {
    fn from(err: String) -> Self {
        ServerError(err)
    }
}

impl From<&str> for ServerError {
    fn from(err: &str) -> Self {
        ServerError(err.into())
    }
}

pub type ServerResult<T> = Result<T, ServerError>;
