#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    LockFailed,
    NoSuchProcess,
    NotAChildProcess,
    InvalidProcessState,
}

pub type Result<T> = core::result::Result<T, Error>;
