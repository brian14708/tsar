#![allow(dead_code)]

mod codec;
mod compress;
mod data_type;
mod paths;
mod read;
mod result;
mod write;

mod pbgen {
    include!(concat!(env!("OUT_DIR"), "/pb/mod.rs"));
}

pub use data_type::DataType;
pub use pbgen::tsar as pb;
pub use read::Archive;
pub use result::{Error, Result};
pub use write::{BlobWriteOption, Builder};
