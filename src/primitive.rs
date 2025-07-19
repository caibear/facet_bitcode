use crate::consume::{consume_byte_arrays, consume_byte_arrays_unchecked};
use crate::error::{err, Result};
use crate::{decoder::Decoder, encoder::Encoder};
use bytemuck::{CheckedBitPattern, NoUninit};
use std::marker::PhantomData;
use std::mem::MaybeUninit;

pub static DUMMY_ENCODER: PrimitiveEncoder<u32> = PrimitiveEncoder(PhantomData);

#[derive(Default)]
pub struct PrimitiveEncoder<T>(PhantomData<fn(T)>);

impl<T: NoUninit> Encoder for PrimitiveEncoder<T> {
    unsafe fn encode_one(&self, erased: *const u8, out: &mut Vec<u8>) {
        let erased: &[u8] =
            std::slice::from_raw_parts(erased as *const u8, std::mem::size_of::<T>());
        out.extend_from_slice(erased); // TODO swap_bytes on big endian.
    }

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

#[derive(Default)]
pub struct PrimitiveDecoder<T>(PhantomData<fn(T)>);

impl<T: CheckedBitPattern> Decoder for PrimitiveDecoder<T> {
    fn validate(&self, input: &mut &[u8], length: usize) -> Result<()> {
        let mut bytes = consume_byte_arrays(input, length, std::mem::size_of::<T>())?;

        // Optimizes much better than Iterator::any.
        if (0..length)
            .filter(|_| {
                let t = unsafe {
                    let mut t: MaybeUninit<T::Bits> = MaybeUninit::uninit();
                    self.decode_one(&mut bytes, t.as_mut_ptr() as *mut u8);
                    t.assume_init()
                };
                !T::is_valid_bit_pattern(&t)
            })
            .count()
            != 0
        {
            return err("invalid bit pattern");
        }
        Ok(())
    }

    unsafe fn decode_one(&self, input: &mut &[u8], erased: *mut u8) {
        let bytes = consume_byte_arrays_unchecked(input, 1, std::mem::size_of::<T>());
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), erased as *mut u8, bytes.len());
        // TODO swap_bytes on big endian.
    }

    unsafe fn decode_many(&self, input: &mut &[u8], erased: *mut [u8]) {
        let bytes = consume_byte_arrays_unchecked(input, erased.len(), std::mem::size_of::<T>());
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), erased as *mut u8, bytes.len());
        // TODO swap_bytes on big endian.
    }
}
