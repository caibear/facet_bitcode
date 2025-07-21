#![cfg_attr(not(feature = "std"), no_std)]
#![feature(alloc_layout_extra)] // TODO remove.
#![cfg_attr(test, feature(test))]
extern crate alloc;
#[cfg(test)]
extern crate test;

#[cfg(test)]
mod benches;
#[cfg(feature = "std")]
mod cache;
mod codec;
mod consume;
mod decoder;
mod deserialize;
mod encoder;
mod error;
mod primitive;
#[rustfmt::skip]
#[allow(clippy::useless_conversion)]
#[allow(clippy::question_mark)]
mod raw_vec_fork;
mod serialize;
mod slice;
mod strided;

pub use crate::error::Error;
pub use deserialize::deserialize;
pub use serialize::{serialize, serialize_into};

#[cfg(feature = "std")]
pub(crate) use cache::reflect;
#[cfg(not(feature = "std"))]
pub(crate) use codec::reflect;
