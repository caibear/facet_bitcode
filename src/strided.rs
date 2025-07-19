use crate::encoder::{try_encode_in_place, Encoder};
use std::alloc::Layout;

pub struct StridedEncoder {
    layout: Layout,
    encoder: Box<dyn Encoder>,
    stride: usize,
    offset: usize,
}

impl StridedEncoder {
    pub fn new(layout: Layout, encoder: Box<dyn Encoder>, stride: usize, offset: usize) -> Self {
        Self {
            layout,
            encoder,
            stride,
            offset,
        }
    }
}

impl Encoder for StridedEncoder {
    unsafe fn encode_one(&self, erased: *const u8, out: &mut Vec<u8>) {
        let erased = erased.byte_add(self.offset);
        self.encoder.encode_one(erased as *const u8, out);
    }

    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>) {
        let erased = erased.byte_add(self.offset);
        let n = erased.len();

        let stride = self.stride;
        let copy_size = self.layout.size();
        let items = (0..n * stride)
            .step_by(stride)
            .map(|i| unsafe { (erased as *const u8).byte_add(i) });

        try_encode_in_place(
            &*self.encoder,
            self.layout,
            n,
            &mut |mut dst| {
                let items = items.clone();
                macro_rules! copy_for_size {
                    ($copy_size:expr) => {
                        for src in items {
                            std::ptr::copy_nonoverlapping(src, dst, $copy_size);
                            dst = dst.byte_add($copy_size);
                        }
                    };
                }

                // Optimize common sizes. TODO optmize for all sizes <= 64, not just powers of 2.
                match copy_size {
                    0 => todo!(),
                    1 => copy_for_size!(1),
                    2 => copy_for_size!(2),
                    4 => copy_for_size!(4),
                    8 => copy_for_size!(8),
                    16 => copy_for_size!(16),
                    32 => copy_for_size!(32),
                    64 => copy_for_size!(64),
                    _ => copy_for_size!(copy_size),
                }
            },
            out,
        );
    }
}

pub struct StructEncoder(Box<[StridedEncoder]>);

impl StructEncoder {
    pub fn new(fields: impl Iterator<Item = StridedEncoder>) -> Self {
        Self(fields.collect())
    }
}

impl Encoder for StructEncoder {
    unsafe fn encode_one(&self, erased: *const u8, out: &mut Vec<u8>) {
        for field in &self.0 {
            field.encode_one(erased, out);
        }
    }

    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>) {
        // TODO needs strip mining optimization to avoid reading the whole struct
        // from memory once per field when it doesn't fit in cache.
        // https://en.wikipedia.org/wiki/Loop_sectioning
        for field in &self.0 {
            field.encode_many(erased, out);
        }
    }

    fn in_place(&self) -> bool {
        false // TODO only 1 field and same layout then can be in_place.
    }
}
