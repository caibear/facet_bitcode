use crate::codec::Codec;
use std::alloc::Layout;

pub trait Encoder: Send + Sync {
    /// Required have the exact same results (but possibly faster) as
    /// `unsafe { codec.encode_many(std::ptr::slice_from_raw_parts(erased, 1), out) };``
    unsafe fn encode_one(&self, erased: *const u8, out: &mut Vec<u8>);

    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>);

    fn in_place(&self) -> bool {
        false
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
    let (dst, staging) = if codec.in_place() {
        out.reserve(dst_size);
        (out.as_mut_ptr_range().end, None)
    } else {
        let (staging_layout, stride) = layout.repeat(n_elements).unwrap();
        debug_assert_eq!(stride, layout.size());
        let staging_elements = std::alloc::alloc(staging_layout); // TODO scratch allocator like rkyv?
        (staging_elements, Some((staging_elements, staging_layout)))
    };

    encode(dst);

    if let Some((staging_elements, staging_layout)) = staging {
        codec.encode_many(std::ptr::slice_from_raw_parts(dst, n_elements), out);
        std::alloc::dealloc(staging_elements, staging_layout);
    } else {
        out.set_len(out.len() + dst_size);
    }
}
