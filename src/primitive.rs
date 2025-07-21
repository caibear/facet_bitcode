use crate::consume::{consume_byte_arrays, consume_byte_arrays_unchecked};
use crate::decoder::Decoder;
use crate::encoder::Encoder;
use crate::error::{err, Result};
use alloc::vec::Vec;
use bytemuck::{CheckedBitPattern, NoUninit};
use core::marker::PhantomData;
use core::mem::MaybeUninit;

#[cfg_attr(not(feature = "std"), allow(unused))]
pub static DUMMY_CODEC: PrimitiveCodec<u32> = PrimitiveCodec(PhantomData);

#[derive(Default)]
pub struct PrimitiveCodec<T>(PhantomData<fn(T)>);

impl<T: NoUninit> Encoder for PrimitiveCodec<T> {
    unsafe fn encode_one(&self, erased: *const u8, out: &mut Vec<u8>) {
        let erased: &[u8] = core::slice::from_raw_parts(erased, core::mem::size_of::<T>());
        out.extend_from_slice(erased); // TODO swap_bytes on big endian.
    }

    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>) {
        let erased: &[u8] = core::slice::from_raw_parts(
            erased as *const u8,
            erased.len() * core::mem::size_of::<T>(),
        );
        out.extend_from_slice(erased); // TODO swap_bytes on big endian.
    }

    fn in_place(&self) -> bool {
        true // TODO only on little endian
    }
}

impl<T: CheckedBitPattern> PrimitiveCodec<T> {
    /// Safety: `bytes` must contain at least enough bytes to decode `length` primitives.
    pub unsafe fn iter<'a>(
        &'a self,
        mut bytes: &'a [u8],
        length: usize,
    ) -> impl Iterator<Item = T::Bits> + 'a {
        (0..length).map(move |_| unsafe {
            let mut t: MaybeUninit<T::Bits> = MaybeUninit::uninit();
            self.decode_one(&mut bytes, t.as_mut_ptr() as *mut u8);
            t.assume_init()
        })
    }
}

impl<T: CheckedBitPattern> Decoder for PrimitiveCodec<T> {
    fn validate(&self, input: &mut &[u8], length: usize) -> Result<()> {
        let bytes = consume_byte_arrays(input, length, core::mem::size_of::<T>())?;

        // Safety: `bytes` contains enough bytes to decode `length` primitives.
        let iter = unsafe { self.iter(bytes, length) };

        // Optimizes much better than Iterator::any.
        if iter.filter(|t| !T::is_valid_bit_pattern(t)).count() != 0 {
            return err("invalid bit pattern");
        }
        Ok(())
    }

    unsafe fn decode_one(&self, input: &mut &[u8], erased: *mut u8) {
        let bytes = consume_byte_arrays_unchecked(input, 1, core::mem::size_of::<T>());
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), erased, bytes.len());
        // TODO swap_bytes on big endian.
    }

    unsafe fn decode_many(&self, input: &mut &[u8], erased: *mut [u8]) {
        let bytes = consume_byte_arrays_unchecked(input, erased.len(), core::mem::size_of::<T>());
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), erased as *mut u8, bytes.len());
        // TODO swap_bytes on big endian.
    }
}
