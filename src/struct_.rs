use crate::codec::DynamicCodec;
use crate::decoder::Decoder;
use crate::encoder::Encoder;
use crate::error::Result;
use alloc::boxed::Box;
use alloc::vec::Vec;

pub struct StructField {
    codec: DynamicCodec,
    offset: usize,
    size: usize,
}

impl StructField {
    pub fn new(codec: DynamicCodec, offset: usize, size: usize) -> Self {
        Self {
            codec,
            offset,
            size,
        }
    }
}

pub struct StructCodec {
    fields: Vec<StructField>,
    size: usize,
}

impl StructCodec {
    pub fn new_dynamic(
        fields_iter: impl Iterator<Item = StructField>,
        size: usize,
    ) -> DynamicCodec {
        let mut fields = Vec::with_capacity(fields_iter.size_hint().0);
        for mut field in fields_iter {
            if let Some(nested) = field.codec.as_struct_codec_mut() {
                fields.extend(core::mem::take(&mut nested.fields).into_iter().map(
                    |nested_field| StructField {
                        offset: nested_field.offset + field.offset,
                        ..nested_field
                    },
                ));
            } else {
                fields.push(field);
            }
        }

        match fields.as_slice() {
            [single] if single.size == size => {
                debug_assert_eq!(single.offset, 0);
                fields.pop().unwrap().codec
            }
            _ => Box::new(Self { fields, size }),
        }
    }
}

impl Encoder for StructCodec {
    unsafe fn encode_one(&self, erased: *const u8, out: &mut Vec<u8>) {
        for field in &self.fields {
            let erased = erased.byte_add(field.offset);
            field.codec.encode_one(erased, out);
        }
    }

    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>) {
        for field in &self.fields {
            let erased = erased.byte_add(field.offset);
            field.codec.encode_many_strided(erased, self.size, out);
        }
    }

    unsafe fn encode_many_strided(&self, _: *const [u8], _: usize, _: &mut Vec<u8>) {
        unreachable!(); // Struct codecs are flattened.
    }

    fn as_struct_codec_mut(&mut self) -> Option<&mut StructCodec> {
        Some(self)
    }
}

impl Decoder for StructCodec {
    fn validate(&self, input: &mut &[u8], length: usize) -> Result<()> {
        for field in &self.fields {
            field.codec.validate(input, length)?;
        }
        Ok(())
    }

    unsafe fn decode_one(&self, input: &mut &[u8], erased: *mut u8) {
        for field in &self.fields {
            let erased = erased.byte_add(field.offset);
            field.codec.decode_one(input, erased);
        }
    }

    unsafe fn decode_many(&self, input: &mut &[u8], erased: *mut [u8]) {
        for field in &self.fields {
            let erased = erased.byte_add(field.offset);
            field.codec.decode_many_strided(input, erased, self.size);
        }
    }

    unsafe fn decode_many_strided(&self, _: &mut &[u8], _: *mut [u8], _: usize) {
        unreachable!(); // Struct codecs are flattened.
    }
}
