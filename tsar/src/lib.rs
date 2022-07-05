#![allow(dead_code)]

mod compress;
mod executor;
mod operator;

pub use compress::*;
pub mod result;
pub mod writer;

mod pb {
    include!(concat!(env!("OUT_DIR"), "/pb/mod.rs"));
}
