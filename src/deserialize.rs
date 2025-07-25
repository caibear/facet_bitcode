use crate::consume::expect_eof;
use crate::error::Error;
use core::mem::MaybeUninit;
use facet_core::Facet;

/// Deserializes a [`&[u8]`][`prim@slice`] into an instance of `T:` [`Facet`].
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
    use crate::benches::Vertex;
    use alloc::vec;
    use core::fmt::Debug;
    use facet::Facet;
    use test::{black_box, Bencher};

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
        roundtrip(&(42,));
        roundtrip(&((42,),));
        roundtrip(&(1, (2, 3)));

        #[derive(Debug, PartialEq, Facet)]
        #[repr(align(8))]
        struct FakeTransparent(u32);
        roundtrip(&vec![FakeTransparent(1), FakeTransparent(2)]);
    }

    #[bench]
    fn bench_decode_u32_facet_bitcode(b: &mut Bencher) {
        let original = 5u32;
        let bytes = crate::serialize(&original);

        b.iter(|| {
            let deserialized: u32 = deserialize(black_box(bytes.as_slice())).unwrap();
            debug_assert_eq!(deserialized, original);
            deserialized
        })
    }

    #[bench]
    fn bench_decode_vertex_facet_bitcode(b: &mut Bencher) {
        let original = Vertex::new(5);
        let bytes = crate::serialize(&original);

        b.iter(|| {
            let deserialized: Vertex = deserialize(black_box(bytes.as_slice())).unwrap();
            debug_assert_eq!(deserialized, original);
            deserialized
        })
    }
}
