#[derive(Debug)]
pub enum Error {
    LockFailed,
}

pub type Result<T> = core::result::Result<T, Error>;
