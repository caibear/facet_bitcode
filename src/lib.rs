#![feature(alloc_layout_extra)] // TODO remove.
#![cfg_attr(test, feature(test))]
extern crate alloc;
#[cfg(test)]
extern crate test;

mod cache;
mod codec;
mod consume;
mod decoder;
mod deserialize;
mod encoder;
mod error;
mod primitive;
#[rustfmt::skip]
mod raw_vec_fork;
mod serialize;
mod slice;
mod strided;

pub use crate::error::Error;
pub use deserialize::deserialize;
pub use serialize::serialize;
