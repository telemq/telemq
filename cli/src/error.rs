#[derive(Debug)]
pub enum ExecutionError {}

impl From<reqwest::Error> for ExecutionError {
    fn from(request_err: reqwest::Error) -> Self {
        println!("REQUEST ERROR {request_err:?}");
        unimplemented!()
    }
}

pub type ExecResult<T> = Result<T, ExecutionError>;
