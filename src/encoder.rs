use crate::codec::Codec;
use crate::struct_::StructCodec;
use alloc::vec::Vec;
use core::alloc::Layout;

pub trait Encoder: Send + Sync {
    /// Required have the exact same results (but possibly faster) as
    /// `unsafe { codec.encode_many(std::ptr::slice_from_raw_parts(erased, 1), out) };``
    unsafe fn encode_one(&self, erased: *const u8, out: &mut Vec<u8>);

    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>);

    unsafe fn encode_many_strided(&self, erased: *const [u8], stride: usize, out: &mut Vec<u8>);

    // TODO used by try_decode_in_place, move to Codec?
    fn in_place(&self) -> bool {
        false
    }

    fn as_struct_codec_mut(&mut self) -> Option<&mut StructCodec> {
        None
    }
}

#[inline(always)]
pub unsafe fn encode_one_or_many(codec: &dyn Codec, erased: *const [u8], out: &mut Vec<u8>) {
    if erased.len() == 1 {
        codec.encode_one(erased as *const u8, out);
    } else {
        codec.encode_many(erased, out);
    }
}

// Uses an FnMut instead of an FnOnce because the latter cannot be called from dyn easily.
#[inline(never)]
pub unsafe fn try_encode_in_place(
    codec: &dyn Codec,
    layout: Layout,
    n_elements: usize,
    encode: &mut dyn FnMut(*mut u8),
    out: &mut Vec<u8>,
) {
    let dst_size = layout.size() * n_elements;
    let (dst, allocation) = if codec.in_place() {
        out.reserve(dst_size);
        (out.as_mut_ptr_range().end, None)
    } else {
        let (allocation, stride) = layout.repeat(n_elements).unwrap();
        debug_assert_eq!(stride, layout.size()); // TODO when can this fail?
        (alloc::alloc::alloc(allocation), Some(allocation)) // TODO scratch allocator like rkyv?
    };

    encode(dst);

    if let Some(allocation) = allocation {
        codec.encode_many(core::ptr::slice_from_raw_parts(dst, n_elements), out);
        alloc::alloc::dealloc(dst, allocation);
    } else {
        out.set_len(out.len() + dst_size);
    }
}
