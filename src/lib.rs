#![feature(alloc_layout_extra)] // TODO remove.
#![cfg_attr(test, feature(test))]
extern crate alloc;
#[cfg(test)]
extern crate test;

mod consume;
mod decoder;
mod deserialize;
mod encoder;
mod error;
mod primitive;
mod serialize;
mod slice;
mod strided;

pub use crate::error::Error;
pub use deserialize::deserialize;
pub use serialize::serialize;
