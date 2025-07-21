use crate::codec::DynamicCodec;
use crate::decoder::{decode_one_or_many, Decoder};
use crate::encoder::{encode_one_or_many, try_encode_in_place, Encoder};
use crate::error::{err, error, Result};
use crate::primitive::PrimitiveCodec;
use crate::raw_vec_fork::RawVecInner;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::mem::MaybeUninit;

type LengthInt = u32; // TODO usize or u64.

pub struct BoxedSliceCodec {
    lengths: PrimitiveCodec<LengthInt>,
    element_layout: Layout,
    elements: DynamicCodec,
}

impl BoxedSliceCodec {
    pub fn new(element_layout: Layout, elements: DynamicCodec) -> Self {
        Self {
            lengths: Default::default(),
            element_layout,
            elements,
        }
    }
}

impl Encoder for BoxedSliceCodec {
    unsafe fn encode_one(&self, erased: *const u8, out: &mut Vec<u8>) {
        let slice = unsafe { *(erased as *const *const [u8]) };
        let len = slice.len() as LengthInt;
        self.lengths
            .encode_one((&len) as *const LengthInt as *const u8, out);
        encode_one_or_many(&*self.elements, slice, out);
    }

    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>) {
        // Using *const [u8] to represent type erased *const [T].
        #[allow(clippy::cast_slice_different_sizes)]
        let erased = erased as *const [*const [u8]];
        let n: usize = erased.len();

        let slices = (0..n).map(|i| unsafe { *(erased as *const *const [u8]).add(i) });
        let mut n_elements = 0;
        try_encode_in_place(
            &self.lengths,
            Layout::for_value(&(0 as LengthInt)),
            n,
            &mut |mut dst| {
                for slice in slices.clone() {
                    n_elements += slice.len();
                    *(dst as *mut LengthInt) = slice.len() as LengthInt;
                    dst = dst.byte_add(core::mem::size_of::<LengthInt>());
                }
            },
            out,
        );

        try_encode_in_place(
            &*self.elements,
            self.element_layout,
            n_elements,
            &mut |mut dst: *mut u8| {
                let element_size = self.element_layout.size();
                for slice in slices.clone() {
                    let slice_len_bytes = slice.len().unchecked_mul(element_size);
                    core::ptr::copy_nonoverlapping(slice as *const u8, dst, slice_len_bytes);
                    dst = dst.byte_add(slice_len_bytes);
                }
            },
            out,
        );
    }
}

impl Decoder for BoxedSliceCodec {
    fn validate(&self, input: &mut &[u8], length: usize) -> Result<()> {
        let before_lengths_consumed = *input;
        self.lengths.validate(input, length)?;
        // Safety: we validated that input contained enough bytes before
        // validate was called, and we use that slice, not the modified input.
        let iter = unsafe { self.lengths.iter(before_lengths_consumed, length) };

        if length > u32::MAX as usize {
            return err("length too large"); // TODO support usize length.
        }
        let mut sum = 0u64;
        for length in iter {
            let length: u32 = length; // If length is changed from u32, this needs to change to be safe.

            // Safety: we checked that there are no more than u32::MAX u32s, and u32::MAX * u32::MAX < u64::MAX.
            sum = unsafe { sum.unchecked_add(length as u64) };
        }
        let sum = sum.try_into().map_err(|_| error("length > usize::MAX"))?;

        self.elements.validate(input, sum)?;
        Ok(())
    }

    unsafe fn decode_one(&self, input: &mut &[u8], erased: *mut u8) {
        let mut length = MaybeUninit::<LengthInt>::uninit();
        self.lengths
            .decode_one(input, length.as_mut_ptr() as *mut u8);
        let length = length.assume_init() as usize;
        let erased_box = allocate_erased_box(length, self.element_layout);
        unsafe { *(erased as *mut *mut [u8]) = erased_box };
        decode_one_or_many(&*self.elements, input, erased_box);
    }

    unsafe fn decode_many(&self, _: &mut &[u8], _: *mut [u8]) {
        todo!();
    }
}

#[inline]
fn allocate_erased_box(length: usize, element_layout: Layout) -> *mut [u8] {
    let erased_raw_vec = RawVecInner::with_capacity(length, element_layout);
    debug_assert_eq!(erased_raw_vec.cap, length); // Current implementation guarantees this.
    core::ptr::slice_from_raw_parts_mut(erased_raw_vec.ptr.as_ptr(), length)
}
