use crate::codec::DynamicCodec;
use crate::decoder::{try_decode_in_place, Decoder};
use crate::encoder::{try_encode_in_place, Encoder};
use crate::error::Result;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::alloc::Layout;

pub struct StridedCodec {
    layout: Layout,
    codec: DynamicCodec,
    stride: usize,
    offset: usize,
}

impl StridedCodec {
    pub fn new(layout: Layout, codec: DynamicCodec, stride: usize, offset: usize) -> Self {
        Self {
            layout,
            codec,
            stride,
            offset,
        }
    }
}

impl Encoder for StridedCodec {
    unsafe fn encode_one(&self, erased: *const u8, out: &mut Vec<u8>) {
        let erased = erased.byte_add(self.offset);
        self.codec.encode_one(erased, out);
    }

    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>) {
        let erased = erased.byte_add(self.offset);

        try_encode_in_place(
            &*self.codec,
            self.layout,
            erased.len(),
            &mut |mut dst| {
                let stride = self.stride;
                let copy_size = self.layout.size();

                let mut ptr = erased as *const u8;
                let items = (0..erased.len()).map(|_| {
                    let p = ptr;
                    unsafe { ptr = ptr.add(stride) };
                    p
                });

                macro_rules! copy_for_size {
                    ($copy_size:expr) => {
                        for src in items {
                            core::ptr::copy_nonoverlapping(src, dst, $copy_size);
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
                    24 => copy_for_size!(24), // Vec<T>
                    32 => copy_for_size!(32),
                    64 => copy_for_size!(64),
                    _ => copy_for_size!(copy_size),
                }
            },
            out,
        );
    }
}

impl Decoder for StridedCodec {
    #[allow(unused)]
    fn validate(&self, input: &mut &[u8], length: usize) -> Result<()> {
        self.codec.validate(input, length)
    }

    #[allow(unused)]
    unsafe fn decode_one(&self, input: &mut &[u8], erased: *mut u8) {
        let erased = erased.byte_add(self.offset);
        self.codec.decode_one(input, erased);
    }

    #[allow(unused)]
    unsafe fn decode_many(&self, input: &mut &[u8], erased: *mut [u8]) {
        let erased = erased.byte_add(self.offset);

        try_decode_in_place(
            &*self.codec,
            self.layout,
            erased.len(),
            &mut |mut src| {
                let stride = self.stride;
                let copy_size = self.layout.size();

                let mut ptr = erased as *mut u8;
                let items = (0..erased.len()).map(|_| {
                    let p = ptr;
                    unsafe { ptr = ptr.add(stride) };
                    p
                });

                macro_rules! copy_for_size {
                    ($copy_size:expr) => {
                        for dst in items {
                            core::ptr::copy_nonoverlapping(src, dst, $copy_size);
                            src = src.byte_add($copy_size);
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
                    24 => copy_for_size!(24), // Vec<T>
                    32 => copy_for_size!(32),
                    64 => copy_for_size!(64),
                    _ => copy_for_size!(copy_size),
                }
            },
            input,
        );
    }
}

pub struct StructCodec(Box<[StridedCodec]>);

impl StructCodec {
    pub fn new(fields: impl Iterator<Item = StridedCodec>) -> Self {
        Self(fields.collect())
    }
}

impl Encoder for StructCodec {
    unsafe fn encode_one(&self, erased: *const u8, out: &mut Vec<u8>) {
        for field in &self.0 {
            field.encode_one(erased, out);
        }
    }

    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>) {
        for field in &self.0 {
            field.encode_many(erased, out);
        }
    }

    fn in_place(&self) -> bool {
        false // TODO only 1 field and same layout then can be in_place.
    }
}

impl Decoder for StructCodec {
    fn validate(&self, input: &mut &[u8], length: usize) -> Result<()> {
        for field in &self.0 {
            field.validate(input, length)?;
        }
        Ok(())
    }

    #[allow(unused)]
    unsafe fn decode_one(&self, input: &mut &[u8], erased: *mut u8) {
        for field in &self.0 {
            field.decode_one(input, erased);
        }
    }

    #[allow(unused)]
    unsafe fn decode_many(&self, input: &mut &[u8], erased: *mut [u8]) {
        for field in &self.0 {
            field.decode_many(input, erased);
        }
    }
}
