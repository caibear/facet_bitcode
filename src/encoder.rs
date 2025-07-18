use std::alloc::Layout;

pub trait Encoder: Send + Sync {
    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>);

    unsafe fn encode(&self, erased: *const u8, out: &mut Vec<u8>) {
        self.encode_many(std::ptr::slice_from_raw_parts(erased, 1), out);
    }

    fn in_place(&self) -> bool {
        false
    }
}

// Uses an FnMut instead of an FnOnce because the latter cannot be called from dyn easily.
#[inline(never)]
pub unsafe fn try_encode_in_place(
    encoder: &dyn Encoder,
    layout: Layout,
    n_elements: usize,
    encode: &mut dyn FnMut(*mut u8),
    out: &mut Vec<u8>,
) {
    let dst_size = layout.size() * n_elements;
    let (dst, staging) = if encoder.in_place() {
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
        encoder.encode_many(std::ptr::slice_from_raw_parts(dst, n_elements), out);
        std::alloc::dealloc(staging_elements, staging_layout);
    } else {
        out.set_len(out.len() + dst_size);
    }
}
