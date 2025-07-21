use alloc::vec;
use alloc::vec::Vec;
use facet_core::Facet;

/// Serializes a `T:` [`Facet`] into a [`Vec<u8>`].
pub fn serialize<'facet, T: Facet<'facet> + ?Sized>(t: &T) -> Vec<u8> {
    let mut out = vec![];
    serialize_into(&mut out, t);
    out
}

/// Serializes a `T:` [`Facet`] directly into a [`&mut Vec<u8>`](`Vec`).
pub fn serialize_into<'facet, T: Facet<'facet> + ?Sized>(out: &mut Vec<u8>, t: &T) {
    let codec = crate::reflect(T::SHAPE);
    unsafe { codec.encode_one(t as *const T as *const u8, out) };
}

#[cfg(test)]
mod tests {
    use super::*;
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
        #[derive(facet::Facet)]
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
}
