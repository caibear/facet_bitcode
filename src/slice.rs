use crate::codec::DynamicCodec;
use crate::decoder::Decoder;
use crate::encoder::{encode_one_or_many, try_encode_in_place, Encoder};
use crate::error::Result;
use crate::primitive::PrimitiveCodec;
use std::alloc::Layout;

type LengthInt = u32; // TODO usize or u64.

pub struct SliceCodec {
    lengths: PrimitiveCodec<LengthInt>,
    element_layout: Layout,
    elements: DynamicCodec,
}

impl SliceCodec {
    pub fn new(element_layout: Layout, elements: DynamicCodec) -> Self {
        Self {
            lengths: Default::default(),
            element_layout,
            elements,
        }
    }
}

impl Encoder for SliceCodec {
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

impl Decoder for SliceCodec {
    fn validate(&self, _: &mut &[u8], _: usize) -> Result<()> {
        slice_decode_panic()
    }

    unsafe fn decode_one(&self, _: &mut &[u8], _: *mut u8) {
        slice_decode_panic()
    }

    unsafe fn decode_many(&self, _: &mut &[u8], _: *mut [u8]) {
        slice_decode_panic()
    }
}

fn slice_decode_panic() -> ! {
    panic!("cannot deserialize &[T]");
}
