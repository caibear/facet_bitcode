use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use facet::Facet;
use serde::{Deserialize, Serialize};
use test::{black_box, Bencher};

#[derive(Debug, PartialEq, Facet, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    r: u8,
    g: u8,
    b: u8,
}

impl Vertex {
    pub fn new(i: usize) -> Self {
        Self {
            x: i as f32,
            y: i as f32,
            z: i as f32,
            r: i as u8,
            g: i as u8,
            b: i as u8,
        }
    }
}

fn mesh(n: usize) -> &'static [Vertex] {
    Vec::leak((0..n).map(Vertex::new).collect())
}

pub fn mesh_one() -> &'static [Vertex] {
    mesh(1)
}

pub fn mesh_1k() -> &'static [Vertex] {
    mesh(1000)
}

macro_rules! bench {
    ($($b:ident),+) => { $(mod $b { use super::*;
    mod serialize { use super::*;
        #[bench]
        fn facet_bitcode(b: &mut Bencher) {
            let v = $b();
            let mut out = vec![];
            b.iter(|| {
                let out = black_box(&mut out);
                out.clear();
                black_box(crate::serialize_into(out, black_box(&v)))
            })
        }

        #[bench]
        fn serde_bitcode(b: &mut Bencher) {
            let v = $b();
            b.iter(|| black_box(bitcode::serialize(black_box(&v))))
        }

        #[bench]
        fn derive_bitcode(b: &mut Bencher) {
            let v = $b();
            let mut buffer = bitcode::Buffer::new();
            b.iter(|| {
                black_box(black_box(&mut buffer).encode(black_box(v)));
            })
        }

        #[bench]
        fn serde_bincode(b: &mut Bencher) {
            let v = $b();
            let mut out = vec![];
            b.iter(|| {
                let out = black_box(&mut out);
                out.clear();
                black_box(bincode::serialize_into(out, black_box(&v)).unwrap())
            })
        }

        #[bench]
        fn facet_xdr(b: &mut Bencher) {
            let v = $b();
            b.iter(|| black_box(facet_xdr::to_vec(black_box(&v)).unwrap()))
        }
    }

    mod deserialize { use super::*;
        #[bench]
        fn facet_bitcode(b: &mut Bencher) {
            let original = $b();
            let bytes = crate::serialize(&original);

            b.iter(|| {
                let deserialized: &'static [Vertex] = crate::deserialize(black_box(bytes.as_slice())).unwrap();
                debug_assert_eq!(deserialized, original);
                // TODO properly implement deserialize Box<[T]> once the facet impl is added.
                let deserialized: Box<[Vertex]> = unsafe { core::mem::transmute(deserialized) };
                deserialized
            })
        }

        #[bench]
        fn serde_bitcode(b: &mut Bencher) {
            let original = $b();
            let bytes = bitcode::serialize(&original).unwrap();

            b.iter(|| {
                let deserialized: Box<[Vertex]> = bitcode::deserialize(black_box(bytes.as_slice())).unwrap();
                debug_assert_eq!(&*deserialized, &*original);
                deserialized
            })
        }

        #[bench]
        fn derive_bitcode(b: &mut Bencher) {
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
        fn serde_bincode(b: &mut Bencher) {
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
        fn facet_xdr(b: &mut Bencher) {
            let original = $b();
            let bytes = facet_xdr::to_vec(&original).unwrap();

            b.iter(|| {
                let deserialized: Vec<Vertex> = facet_xdr::deserialize(black_box(bytes.as_slice())).unwrap();
                // xdr current serializes u8 as u64 and deserializes it as u32, but perf should be similar.
                // debug_assert_eq!(&*deserialized, &*original);
                deserialized
            })
        }
    }
})+}}
bench!(mesh_one, mesh_1k);
