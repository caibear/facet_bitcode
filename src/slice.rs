use crate::encoder::{try_encode_in_place, Encoder};
use crate::primitive::PrimitiveEncoder;
use std::alloc::Layout;

pub struct SliceEncoder {
    lengths: PrimitiveEncoder<u32>,
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
        let n = erased.len();
        let slices = (0..n).map(|i| unsafe { *(erased as *const *const [u8]).add(i) });

        let mut n_elements = 0;
        try_encode_in_place(
            &self.lengths,
            Layout::for_value(&0u32),
            n,
            &mut |mut dst| {
                for slice in slices.clone() {
                    n_elements += slice.len();
                    *(dst as *mut u32) = slice.len() as u32; // TODO usize.
                    dst = dst.byte_add(std::mem::size_of::<u32>());
                }
            },
            out,
        );

        // TODO if there's just one slice it can be used directly without copying.
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
