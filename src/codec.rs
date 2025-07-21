use crate::decoder::Decoder;
use crate::encoder::Encoder;
use crate::primitive::PrimitiveCodec;
use crate::slice::BoxedSliceCodec;
use crate::strided::{StridedCodec, StructCodec};
use alloc::boxed::Box;
use bytemuck::{CheckedBitPattern, NoUninit};
use facet_core::{
    Def, KnownPointer, NumericType, PointerDef, PointerType, PrimitiveType, SequenceType, Shape,
    SliceType, TextualType, Type, UserType, ValuePointerType,
};

pub trait Codec: Encoder + Decoder {}
impl<T: Encoder + Decoder> Codec for T {}
pub type DynamicCodec = Box<dyn Codec>;

fn primitive<T: NoUninit + CheckedBitPattern + Default>() -> DynamicCodec {
    Box::new(PrimitiveCodec::<T>::default())
}

pub fn reflect(shape: &Shape) -> DynamicCodec {
    match shape.ty {
        Type::Primitive(PrimitiveType::Numeric(NumericType::Integer { signed: false })) => {
            match shape.layout.sized_layout().unwrap().size() {
                1 => primitive::<u8>(),
                2 => primitive::<u16>(),
                4 => primitive::<u32>(),
                8 => primitive::<u64>(),
                // TODO detect usize.
                _ => todo!("{shape:?}"),
            }
        }
        Type::Primitive(PrimitiveType::Numeric(NumericType::Integer { signed: true })) => {
            match shape.layout.sized_layout().unwrap().size() {
                1 => primitive::<i8>(),
                2 => primitive::<i16>(),
                4 => primitive::<i32>(),
                8 => primitive::<i64>(),
                // TODO detect isize.
                _ => todo!("{shape:?}"),
            }
        }
        Type::Primitive(PrimitiveType::Numeric(NumericType::Float)) => {
            match shape.layout.sized_layout().unwrap().size() {
                4 => primitive::<f32>(),
                8 => primitive::<f64>(),
                _ => todo!("{shape:?}"),
            }
        }
        Type::Primitive(PrimitiveType::Boolean) => primitive::<bool>(),
        Type::Primitive(PrimitiveType::Textual(TextualType::Char)) => primitive::<char>(),
        // TODO(safety) packed struct
        Type::User(UserType::Struct(t)) => {
            Box::new(StructCodec::new(t.fields.iter().map(|field| {
                // TODO respect field.flags
                StridedCodec::new(
                    field.shape.layout.sized_layout().unwrap(),
                    reflect(field.shape),
                    shape.layout.sized_layout().unwrap().size(),
                    field.offset,
                )
            })))
        }
        Type::User(UserType::Opaque) => {
            match shape.def {
                Def::Pointer(PointerDef {
                    known: Some(KnownPointer::Box),
                    pointee: Some(pointee),
                    ..
                }) => {
                    match pointee().ty {
                        Type::Sequence(SequenceType::Slice(SliceType { t })) => {
                            let _ = t;
                            // TODO Facet isn't implemented on Box<[T]> yet.
                            todo!("Box<[T]> is ready to be implemented {shape:?}");
                        }
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
            // TODO unsound for testing, shouldn't be able to decode &[T], only Box<[T]>.
            Type::Sequence(SequenceType::Slice(SliceType { t })) => Box::new(BoxedSliceCodec::new(
                t.layout.sized_layout().unwrap(),
                reflect(t),
            )),
            _ => todo!("{shape:?}"),
        },
        _ => todo!("{shape:?}"),
    }
}
