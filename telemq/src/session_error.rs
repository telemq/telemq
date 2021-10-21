use std::sync::PoisonError;

/// Error structure associated with `Session`.
#[derive(Debug)]
pub struct SessionError {
    pub kind: SessionErrorKind,
    pub description: String,
}

impl SessionError {
    pub fn new<D: ToString>(kind: SessionErrorKind, description: D) -> SessionError {
        SessionError {
            kind,
            description: description.to_string(),
        }
    }
}

impl<T> From<PoisonError<T>> for SessionError {
    fn from(err: PoisonError<T>) -> SessionError {
        SessionError::new(SessionErrorKind::MutexError, format!("{:?}", err))
    }
}

#[derive(Debug)]
pub enum SessionErrorKind {
    WrongState,
    MqttPolicyError,
    TransactionError,
    MutexError,
}

pub type SessionResult<R> = Result<R, SessionError>;
