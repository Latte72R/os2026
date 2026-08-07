#[derive(Debug)]
pub enum Error {
    LockFailed,
    NoSuchProcess,
    NotAChildProcess,
}

pub type Result<T> = core::result::Result<T, Error>;
