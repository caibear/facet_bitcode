use crate::consume::expect_eof;
use crate::error::Error;
use core::mem::MaybeUninit;
use facet_core::Facet;

pub fn deserialize<'facet, T: Facet<'facet>>(bytes: &[u8]) -> Result<T, Error> {
    let codec = crate::reflect(T::SHAPE);

    let mut validated = bytes;
    codec.validate(&mut validated, 1)?;
    expect_eof(validated)?;

    let mut uninit = MaybeUninit::<T>::uninit();
    let mut decoded = bytes;
    unsafe { codec.decode_one(&mut decoded, uninit.as_mut_ptr() as *mut u8) };
    // Important assertion, validate and decode should consume the exact same amount of input.
    debug_assert_eq!(validated.len(), decoded.len());

    unsafe { Ok(uninit.assume_init()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::tests::*;
    use alloc::boxed::Box;
    use alloc::vec;
    use alloc::vec::Vec;
    use core::fmt::Debug;

    fn roundtrip<'facet, T: Facet<'facet> + Debug + PartialEq>(t: &T) {
        let bytes = crate::serialize(t);
        let deserialized = crate::deserialize::<T>(&bytes).expect(core::any::type_name::<T>());
        assert_eq!(t, &deserialized);
    }

    #[test]
    fn test_deserialize_primitives() {
        roundtrip(&5u8);
        roundtrip(&5u16);
        roundtrip(&5u32);
        roundtrip(&5u64);

        roundtrip(&-5i8);
        roundtrip(&-5i16);
        roundtrip(&-5i32);
        roundtrip(&-5i64);

        roundtrip(&5f32);
        roundtrip(&5f64);

        roundtrip(&false);
        roundtrip(&true);

        roundtrip(&'a');
    }

    #[test]
    fn test_invalid_bool() {
        assert!(crate::deserialize::<bool>(&crate::serialize(&2u8)).is_err());
    }

    #[test]
    fn test_invalid_char() {
        assert!(crate::deserialize::<char>(&crate::serialize(&u32::MAX)).is_err());
        assert!(crate::deserialize::<char>(&crate::serialize(&(0xD800u32 - 1))).is_ok());
        assert!(crate::deserialize::<char>(&crate::serialize(&0xD800u32)).is_err());
        assert!(crate::deserialize::<char>(&crate::serialize(&0xDFFFu32)).is_err());
        assert!(crate::deserialize::<char>(&crate::serialize(&(0xDFFFu32 + 1))).is_ok());
    }

    // TODO re-add.
    /*#[test]
    #[should_panic = "cannot deserialize &[T]"]
    fn test_invalid_deserialize_slice() {
        let _ = crate::deserialize::<&[&[u8]]>(&[]);
    }*/

    #[test]
    fn test_struct() {
        roundtrip(&Vertex::new(42));
    }

    #[bench]
    fn bench_decode_u32_facet_derive(b: &mut Bencher) {
        let original = 5u32;
        let bytes = crate::serialize(&original);

        b.iter(|| {
            let deserialized: u32 = deserialize(black_box(bytes.as_slice())).unwrap();
            debug_assert_eq!(deserialized, original);
            deserialized
        })
    }

    #[bench]
    fn bench_decode_vertex_facet_derive(b: &mut Bencher) {
        let original = Vertex::new(5);
        let bytes = crate::serialize(&original);

        b.iter(|| {
            let deserialized: Vertex = deserialize(black_box(bytes.as_slice())).unwrap();
            debug_assert_eq!(deserialized, original);
            deserialized
        })
    }

    macro_rules! bench {
        ($($b:ident),+) => { $(paste::paste! {
            #[bench]
            fn [<bench_deserialize_ $b _facet_bitcode>](b: &mut Bencher) {
                let original = $b();
                let bytes = crate::serialize(&original);

                b.iter(|| {
                    let deserialized: &'static [Vertex] = deserialize(black_box(bytes.as_slice())).unwrap();
                    debug_assert_eq!(deserialized, original);
                    // TODO properly implement deserialize Box<[T]> once the facet impl is added.
                    let deserialized: Box<[Vertex]> = unsafe { core::mem::transmute(deserialized) };
                    deserialized
                })
            }

            #[bench]
            fn [<bench_deserialize_ $b _serde_bitcode>](b: &mut Bencher) {
                let original = $b();
                let bytes = bitcode::serialize(&original).unwrap();

                b.iter(|| {
                    let deserialized: Box<[Vertex]> = bitcode::deserialize(black_box(bytes.as_slice())).unwrap();
                    debug_assert_eq!(&*deserialized, &*original);
                    deserialized
                })
            }

            #[bench]
            fn [<bench_deserialize_ $b _derive_bitcode>](b: &mut Bencher) {
                let mut buffer = bitcode::Buffer::new();

                let original = $b();
                let bytes = buffer.encode(original).to_vec();

                b.iter(|| {
                    let deserialized: Box<[Vertex]> = buffer.decode(black_box(bytes.as_slice())).unwrap();
                    debug_assert_eq!(&*deserialized, &*original);
                    deserialized
                })
            }

            #[bench]
            fn [<bench_deserialize_ $b _bincode>](b: &mut Bencher) {
                let original = $b();
                let mut bytes = vec![];
                bincode::serialize_into(&mut bytes, &original).unwrap();

                b.iter(|| {
                    let deserialized: Box<[Vertex]> = bincode::deserialize(black_box(bytes.as_slice())).unwrap();
                    debug_assert_eq!(&*deserialized, &*original);
                    deserialized
                })
            }

            #[bench]
            fn [<bench_deserialize_ $b _facet_xdr>](b: &mut Bencher) {
                let original = $b();
                let bytes = facet_xdr::to_vec(&original).unwrap();

                b.iter(|| {
                    let deserialized: Vec<Vertex> = facet_xdr::deserialize(black_box(bytes.as_slice())).unwrap();
                    // xdr current serializes u8 as u64 and deserializes it as u32, but perf should be similar.
                    // debug_assert_eq!(&*deserialized, &*original);
                    deserialized
                })
            }
        })+};
    }

    bench!(mesh_one, mesh_1k);
}
