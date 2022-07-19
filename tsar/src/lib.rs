#![allow(dead_code)]

mod codec;
mod compress;
mod data_type;
pub mod result;
pub mod write;

mod pb {
    include!(concat!(env!("OUT_DIR"), "/pb/mod.rs"));
}

pub use data_type::DataType;
