use crate::decoder::Decoder;
use crate::encoder::Encoder;
use crate::primitive::{PrimitiveDecoder, PrimitiveEncoder, DUMMY_ENCODER};
use crate::slice::SliceEncoder;
use crate::strided::{StridedEncoder, StructEncoder};
use bytemuck::{CheckedBitPattern, NoUninit};
use facet_core::{
    Def, KnownPointer, NumericType, PointerDef, PointerType, PrimitiveType, SequenceType, Shape,
    SliceType, TextualType, Type, UserType, ValuePointerType,
};

pub static _DUMMY_CODEC: StaticCodec = StaticCodec {
    encoder: &DUMMY_ENCODER,
    decoder: None,
};

#[derive(Copy, Clone)]
pub struct StaticCodec {
    pub encoder: &'static dyn Encoder,
    pub decoder: Option<&'static dyn Decoder>,
}

impl StaticCodec {
    pub fn new(shape: &'static Shape) -> Self {
        let Codec { encoder, decoder } = Codec::new(shape);
        Self {
            encoder: Box::leak(encoder),
            decoder: decoder.map(|v| &*Box::leak(v)),
        }
    }
}

struct Codec {
    encoder: Box<dyn Encoder>,
    decoder: Option<Box<dyn Decoder>>, // option because e.g. &[T] can't be deserialized
}

impl Codec {
    fn primitive<T: NoUninit + CheckedBitPattern + Default>() -> Self {
        Self {
            encoder: Box::new(PrimitiveEncoder::<T>::default()),
            decoder: Some(Box::new(PrimitiveDecoder::<T>::default())),
        }
    }

    fn new(shape: &'static Shape) -> Self {
        match shape.ty {
            Type::Primitive(PrimitiveType::Numeric(NumericType::Integer { signed: false })) => {
                match shape.layout.sized_layout().unwrap().size() {
                    1 => Self::primitive::<u8>(),
                    2 => Self::primitive::<u16>(),
                    4 => Self::primitive::<u32>(),
                    8 => Self::primitive::<u64>(),
                    // TODO detect usize.
                    _ => todo!("{shape:?}"),
                }
            }
            Type::Primitive(PrimitiveType::Numeric(NumericType::Integer { signed: true })) => {
                match shape.layout.sized_layout().unwrap().size() {
                    1 => Self::primitive::<i8>(),
                    2 => Self::primitive::<i16>(),
                    4 => Self::primitive::<i32>(),
                    8 => Self::primitive::<i64>(),
                    // TODO detect isize.
                    _ => todo!("{shape:?}"),
                }
            }
            Type::Primitive(PrimitiveType::Numeric(NumericType::Float)) => {
                match shape.layout.sized_layout().unwrap().size() {
                    4 => Self::primitive::<f32>(),
                    8 => Self::primitive::<f64>(),
                    _ => todo!("{shape:?}"),
                }
            }
            Type::Primitive(PrimitiveType::Boolean) => Self::primitive::<bool>(),
            Type::Primitive(PrimitiveType::Textual(TextualType::Char)) => Self::primitive::<char>(),
            // TODO(safety) packed struct
            Type::User(UserType::Struct(t)) => {
                Self {
                    encoder: Box::new(StructEncoder::new(t.fields.iter().map(|field| {
                        // TODO respect field.flags
                        StridedEncoder::new(
                            field.shape.layout.sized_layout().unwrap(),
                            Self::new(field.shape).encoder,
                            shape.layout.sized_layout().unwrap().size(),
                            field.offset,
                        )
                    }))),
                    decoder: None, // TODO struct decoder.
                }
            }
            Type::User(UserType::Opaque) => {
                match shape.def {
                    Def::Pointer(PointerDef {
                        known: Some(KnownPointer::Box),
                        pointee: Some(pointee),
                        ..
                    }) => {
                        match pointee().ty {
                            // Box<[T]> and &[T] have equivilant reprs so this is safe.
                            // TODO Facet isn't implemented on Box<[T]> yet.
                            Type::Sequence(SequenceType::Slice(SliceType { t })) => Self {
                                encoder: Box::new(SliceEncoder::new(
                                    t.layout.sized_layout().unwrap(),
                                    Self::new(t).encoder,
                                )),
                                decoder: None, // TODO slice decoder.
                            },
                            _ => todo!("{shape:?}"),
                        }
                    }
                    _ => todo!("{shape:?}"),
                }
            }
            Type::Pointer(PointerType::Reference(ValuePointerType {
                mutable: false,
                wide: true,
                target,
            })) => match target().ty {
                Type::Sequence(SequenceType::Slice(SliceType { t })) => Self {
                    encoder: Box::new(SliceEncoder::new(
                        t.layout.sized_layout().unwrap(),
                        Self::new(t).encoder,
                    )),
                    decoder: None, // &[T] can't be decoded.
                },
                _ => todo!("{shape:?}"),
            },
            _ => todo!("{shape:?}"),
        }
    }
}
