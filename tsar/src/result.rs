use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown error")]
    Unknown,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
