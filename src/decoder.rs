use crate::codec::Codec;
use crate::consume::consume_byte_arrays_unchecked;
use crate::error::Result;
use std::alloc::Layout;

pub trait Decoder: Send + Sync {
    /// Validates that enough bytes are present and that they
    /// don't contain invalid values for e.g. bool or char.
    ///
    /// needs to happen before decoding for two reasons:
    /// 1. so we don't allocate memory for elements that don't exist
    /// 2. so we don't have to implement dropping a partially initalized output
    fn validate(&self, input: &mut &[u8], length: usize) -> Result<()>;

    /// Required to have the exact same results (but possibly faster) as
    /// `unsafe { decoder.decode_many(input, std::ptr::slice_from_raw_parts_mut(erased, 1)) };`
    /// Safety: ^^^
    unsafe fn decode_one(&self, input: &mut &[u8], erased: *mut u8);

    /// TODO(optimization) use a structure that can avoid mutating length such as a slice iterator or a pointer.
    /// Safety: validate must have succeded with the same parameters.
    unsafe fn decode_many(&self, input: &mut &[u8], erased: *mut [u8]);
}

#[inline(always)]
pub unsafe fn decode_one_or_many(codec: &dyn Codec, input: &mut &[u8], erased: *mut [u8]) {
    if erased.len() == 1 {
        codec.decode_one(input, erased as *mut u8);
    } else {
        codec.decode_many(input, erased);
    }
}

#[inline(never)]
pub unsafe fn try_decode_in_place(
    codec: &dyn Codec,
    layout: Layout,
    n_elements: usize,
    decode: &mut dyn FnMut(*const u8),
    input: &mut &[u8],
) {
    let (src, allocation) = if codec.in_place() {
        let src = consume_byte_arrays_unchecked(input, n_elements, layout.size()).as_ptr();
        (src, None)
    } else {
        let (allocation, stride) = layout.repeat(n_elements).unwrap();
        debug_assert_eq!(stride, layout.size());
        let src = std::alloc::alloc(allocation); // TODO scratch allocator like rkyv?
        codec.decode_many(input, std::ptr::slice_from_raw_parts_mut(src, n_elements));
        (src as *const u8, Some(allocation))
    };

    decode(src);

    if let Some(allocation) = allocation {
        std::alloc::dealloc(src as *mut u8, allocation);
    }
}
