use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ProtoBuf error: {0}")]
    Protobuf(#[from] protobuf::Error),
    #[error("ZPF error")]
    ZPFUnknown,
    #[error("unknown error")]
    Unknown,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
