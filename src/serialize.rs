use crate::encoder::{encode_one, Encoder};
use crate::primitive::PrimitiveEncoder;
use crate::slice::SliceEncoder;
use crate::strided::{StridedEncoder, StructEncoder};
use facet_core::{
    ConstTypeId, Def, Facet, KnownPointer, NumericType, PointerDef, PointerType, PrimitiveType,
    SequenceType, Shape, SliceType, TextualType, Type, UserType, ValuePointerType,
};
use std::sync::RwLock;

pub fn serialize<'facet, T: Facet<'facet> + ?Sized>(t: &T) -> Vec<u8> {
    let mut out = vec![];
    serialize_into(&mut out, t);
    out
}

fn serialize_into<'facet, T: Facet<'facet> + ?Sized>(out: &mut Vec<u8>, t: &T) {
    let encoder = encoder_cached(T::SHAPE);
    unsafe { encode_one(encoder, t as *const T as *const u8, out) };
}

// TODO once cache with Encoder + Option<Decoder>.
// TODO replace with thread_local to avoid contention?
static ENCODER_CACHE: RwLock<Vec<(ConstTypeId, &'static dyn Encoder)>> = RwLock::new(vec![]);

fn encoder_cached(shape: &'static Shape) -> &'static dyn Encoder {
    let read_cache = ENCODER_CACHE.read().unwrap();
    // TODO use binary search.
    if let Some((_, encoder)) = read_cache.iter().copied().find(|(id, _)| *id == shape.id) {
        return encoder;
    }
    drop(read_cache);
    let mut write_cache = ENCODER_CACHE.write().unwrap();
    match write_cache.binary_search_by_key(&shape.id, |(id, _)| *id) {
        Ok(i) => write_cache[i].1,
        Err(i) => {
            // TODO a Vec<T> encoder could share the T encoder if encoders contained static references.
            let encoder = Box::leak(encoder(shape));
            write_cache.insert(i, (shape.id, encoder));
            encoder
        }
    }
}

fn encoder(shape: &'static Shape) -> Box<dyn Encoder> {
    match shape.ty {
        Type::Primitive(PrimitiveType::Numeric(NumericType::Integer { signed: false })) => {
            match shape.layout.sized_layout().unwrap().size() {
                1 => Box::new(PrimitiveEncoder::<u8>::default()),
                2 => Box::new(PrimitiveEncoder::<u16>::default()),
                4 => Box::new(PrimitiveEncoder::<u32>::default()),
                8 => Box::new(PrimitiveEncoder::<u64>::default()),
                // TODO detect usize.
                _ => todo!("{shape:?}"),
            }
        }
        Type::Primitive(PrimitiveType::Numeric(NumericType::Integer { signed: true })) => {
            match shape.layout.sized_layout().unwrap().size() {
                1 => Box::new(PrimitiveEncoder::<i8>::default()),
                2 => Box::new(PrimitiveEncoder::<i16>::default()),
                4 => Box::new(PrimitiveEncoder::<i32>::default()),
                8 => Box::new(PrimitiveEncoder::<i64>::default()),
                // TODO detect isize.
                _ => todo!("{shape:?}"),
            }
        }
        Type::Primitive(PrimitiveType::Numeric(NumericType::Float)) => {
            match shape.layout.sized_layout().unwrap().size() {
                4 => Box::new(PrimitiveEncoder::<f32>::default()),
                8 => Box::new(PrimitiveEncoder::<f64>::default()),
                _ => todo!("{shape:?}"),
            }
        }
        Type::Primitive(PrimitiveType::Boolean) => Box::new(PrimitiveEncoder::<bool>::default()),
        Type::Primitive(PrimitiveType::Textual(TextualType::Char)) => {
            Box::new(PrimitiveEncoder::<char>::default())
        }
        // TODO(safety) packed struct
        Type::User(UserType::Struct(t)) => {
            Box::new(StructEncoder::new(t.fields.iter().map(|field| {
                // TODO respect field.flags
                StridedEncoder::new(
                    field.shape.layout.sized_layout().unwrap(),
                    encoder(field.shape),
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
                        // Box<[T]> and &[T] have equivilant reprs so this is safe.
                        // TODO Facet isn't implemented on Box<[T]> yet.
                        Type::Sequence(SequenceType::Slice(SliceType { t })) => Box::new(
                            SliceEncoder::new(t.layout.sized_layout().unwrap(), encoder(t)),
                        ),
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
            Type::Sequence(SequenceType::Slice(SliceType { t })) => Box::new(SliceEncoder::new(
                t.layout.sized_layout().unwrap(),
                encoder(t),
            )),
            _ => todo!("{shape:?}"),
        },
        _ => todo!("{shape:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use facet::Facet;
    use serde::Serialize;
    use test::{black_box, Bencher};

    #[test]
    fn test_serialize_primitives() {
        assert_eq!(serialize(&5u8), vec![5]);
        assert_eq!(serialize(&5u16), vec![5, 0]);
        assert_eq!(serialize(&5u32), vec![5, 0, 0, 0]);
        assert_eq!(serialize(&5u64), vec![5, 0, 0, 0, 0, 0, 0, 0]);

        assert_eq!(serialize(&-5i8), vec![251]);
        assert_eq!(serialize(&-5i16), vec![251, 255]);
        assert_eq!(serialize(&-5i32), vec![251, 255, 255, 255]);
        assert_eq!(
            serialize(&-5i64),
            vec![251, 255, 255, 255, 255, 255, 255, 255]
        );

        assert_eq!(serialize(&5f32), 5f32.to_bits().to_le_bytes());
        assert_eq!(serialize(&5f64), 5f64.to_bits().to_le_bytes());

        assert_eq!(serialize(&false), vec![0]);
        assert_eq!(serialize(&true), vec![1]);

        assert_eq!(serialize(&'a'), ('a' as u32).to_le_bytes());
    }

    #[test]
    fn test_serialize_slice_u32() {
        let v = [5u32].as_slice();
        let out = serialize(&v);
        assert_eq!(out, vec![1, 0, 0, 0, 5, 0, 0, 0]);
    }

    #[test]
    fn test_serialize_double_nested_slice_u32() {
        let v0 = [5u32, 6u32];
        let v1 = [v0.as_slice(), v0.as_slice()];
        let v = v1.as_slice();
        let out = serialize(&v);
        assert_eq!(
            out,
            vec![
                2, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 5, 0, 0, 0, 6, 0, 0, 0, 5, 0, 0, 0, 6, 0, 0, 0
            ]
        );
    }

    #[test]
    fn test_serialize_triple_nested_slice_u32() {
        let v0 = [5u32];
        let v1 = [v0.as_slice()];
        let v2 = [v1.as_slice()];
        let v = v2.as_slice();
        let out = serialize(&v);
        assert_eq!(out, vec![1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 5, 0, 0, 0]);
    }

    #[test]
    fn test_serialize_struct() {
        #[derive(Facet)]
        struct Foo(u32, u8, bool);

        let out = serialize(&Foo(3, 2, true));
        assert_eq!(out, vec![3, 0, 0, 0, 2, 1]);

        let out = serialize(&[Foo(33, 3, true), Foo(22, 2, false), Foo(11, 1, true)].as_slice());
        assert_eq!(
            out,
            vec![3, 0, 0, 0, 33, 0, 0, 0, 22, 0, 0, 0, 11, 0, 0, 0, 3, 2, 1, 1, 0, 1]
        );
    }

    fn nested_slice() -> &'static [&'static [&'static [&'static [&'static [&'static [&'static [&'static [&'static [&'static [u16]]]]]]]]]]{
        let depth = 4;
        let n = 40;
        let n = |d| {
            if d <= depth {
                n
            } else {
                1usize
            }
        };

        let mut iter = 0..u64::MAX;
        let mut v0 = vec![];
        for _ in 0..n(10) {
            let mut v1 = vec![];
            for _ in 0..n(9) {
                let mut v2 = vec![];
                for _ in 0..n(8) {
                    let mut v3 = vec![];
                    for _ in 0..n(7) {
                        let mut v4 = vec![];
                        for _ in 0..n(6) {
                            let mut v5 = vec![];
                            for _ in 0..n(5) {
                                let mut v6 = vec![];
                                for _ in 0..n(4) {
                                    let mut v7 = vec![];
                                    for _ in 0..n(3) {
                                        let mut v8 = vec![];
                                        for _ in 0..n(2) {
                                            let mut v9 = vec![];
                                            for _ in 0..n(1) {
                                                v9.push(iter.next().unwrap() as u16);
                                            }
                                            v8.push(&*v9.leak());
                                        }
                                        v7.push(&*v8.leak());
                                    }
                                    v6.push(&*v7.leak());
                                }
                                v5.push(&*v6.leak());
                            }
                            v4.push(&*v5.leak());
                        }
                        v3.push(&*v4.leak());
                    }
                    v2.push(&*v3.leak());
                }
                v1.push(&*v2.leak());
            }
            v0.push(&*v1.leak());
        }
        &*v0.leak()
    }

    #[bench]
    fn bench_nested_slice_facet_bitcode(b: &mut Bencher) {
        let v = nested_slice();
        b.iter(|| black_box(serialize(black_box(&v))))
    }

    #[bench]
    fn bench_nested_slice_bincode(b: &mut Bencher) {
        let v = nested_slice();
        b.iter(|| black_box(bincode::serialize(black_box(&v)).unwrap()))
    }

    #[bench]
    fn bench_nested_slice_facet_xdr(b: &mut Bencher) {
        let v = nested_slice();
        b.iter(|| black_box(facet_xdr::to_vec(black_box(&v)).unwrap()))
    }

    macro_rules! bench {
        ($($b:ident),+) => { $(paste::paste! {
            #[bench]
            fn [<bench_ $b _facet_bitcode>](b: &mut Bencher) {
                let v = $b();
                let mut out = vec![];
                b.iter(|| {
                    let out = black_box(&mut out);
                    out.clear();
                    black_box(serialize_into(out, black_box(&v)))
                })
            }

            #[bench]
            fn [<bench_ $b _serde_bitcode>](b: &mut Bencher) {
                let v = $b();
                b.iter(|| black_box(bitcode::serialize(black_box(&v))))
            }

            #[bench]
            fn [<bench_ $b _derive_bitcode>](b: &mut Bencher) {
                let v = $b();
                let mut buffer = bitcode::Buffer::new();
                b.iter(|| {
                    black_box(black_box(&mut buffer).encode(black_box(v)));
                })
            }

            #[bench]
            fn [<bench_ $b _bincode>](b: &mut Bencher) {
                let v = $b();
                let mut out = vec![];
                b.iter(|| {
                    let out = black_box(&mut out);
                    out.clear();
                    black_box(bincode::serialize_into(out, black_box(&v)).unwrap())
                })
            }

            #[bench]
            fn [<bench_ $b _facet_xdr>](b: &mut Bencher) {
                let v = $b();
                b.iter(|| black_box(facet_xdr::to_vec(black_box(&v)).unwrap()))
            }
        })+};
    }

    #[derive(Facet, Serialize, bitcode::Encode)]
    struct Vertex {
        x: f32,
        y: f32,
        z: f32,
        r: u8,
        g: u8,
        b: u8,
    }

    fn mesh(n: usize) -> &'static [Vertex] {
        Vec::leak(
            (0..n)
                .map(|i| Vertex {
                    x: i as f32,
                    y: i as f32,
                    z: i as f32,
                    r: i as u8,
                    g: i as u8,
                    b: i as u8,
                })
                .collect(),
        )
    }

    fn mesh_one() -> &'static [Vertex] {
        mesh(1)
    }

    fn mesh_1k() -> &'static [Vertex] {
        mesh(1000)
    }
    bench!(mesh_one, mesh_1k);
}
