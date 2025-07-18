use crate::encoder::Encoder;
use bytemuck::NoUninit;
use std::marker::PhantomData;

#[derive(Default)]
pub struct PrimitiveEncoder<T>(PhantomData<fn(T)>);

impl<T: NoUninit> Encoder for PrimitiveEncoder<T> {
    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>) {
        let erased: &[u8] = std::slice::from_raw_parts(
            erased as *const u8,
            erased.len() * std::mem::size_of::<T>(),
        );
        out.extend_from_slice(erased); // TODO swap_bytes on big endian.
    }

    fn in_place(&self) -> bool {
        true // TODO only on little endian
    }
}
