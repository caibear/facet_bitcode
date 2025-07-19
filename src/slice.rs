use crate::encoder::{encode_one, try_encode_in_place, Encoder};
use crate::primitive::PrimitiveEncoder;
use std::alloc::Layout;

type LengthInt = u32; // TODO usize or u64.

pub struct SliceEncoder {
    lengths: PrimitiveEncoder<LengthInt>,
    element_layout: Layout,
    elements: Box<dyn Encoder>,
}

impl SliceEncoder {
    pub fn new(element_layout: Layout, elements: Box<dyn Encoder>) -> Self {
        Self {
            lengths: Default::default(),
            element_layout,
            elements,
        }
    }
}

impl Encoder for SliceEncoder {
    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>) {
        let erased = erased as *const [*const [u8]];
        let n: usize = erased.len();
        // Optimization: if there's only 1 slice we don't have to concatenate slices.
        if n == 1 {
            let slice = unsafe { *(erased as *const *const [u8]) };
            let len = slice.len() as LengthInt;
            encode_one(&self.lengths, (&len) as *const LengthInt as *const u8, out);
            self.elements.encode_many(slice, out);
            return;
        }

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
                    dst = dst.byte_add(std::mem::size_of::<LengthInt>());
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
                    std::ptr::copy_nonoverlapping(slice as *const u8, dst, slice_len_bytes);
                    dst = dst.byte_add(slice_len_bytes);
                }
            },
            out,
        );
    }
}
