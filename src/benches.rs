use alloc::vec;
use test::{black_box, Bencher};

mod log;
use log::{log_1k, log_one, Log};
mod mesh;
pub use mesh::Vertex;
use mesh::{mesh_1k, mesh_one, Mesh};

macro_rules! bench {
    ($($b:ident: $t:ty),+) => { $(mod $b { use super::*;
    mod serialize { use super::*;
        #[bench]
        fn facet_bitcode(b: &mut Bencher) {
            let v: $t = $b();
            let mut out = vec![];
            b.iter(|| {
                let out = black_box(&mut out);
                out.clear();
                black_box(crate::serialize_into(out, black_box(&v)))
            })
        }

        #[bench]
        fn serde_bitcode(b: &mut Bencher) {
            let v: $t = $b();
            b.iter(|| black_box(bitcode::serialize(black_box(&v))))
        }

        #[bench]
        fn derive_bitcode(b: &mut Bencher) {
            let v: $t = $b();
            let mut buffer = bitcode::Buffer::new();
            b.iter(|| {
                black_box(black_box(&mut buffer).encode(black_box(&v)));
            })
        }

        #[bench]
        fn serde_bincode(b: &mut Bencher) {
            let v: $t = $b();
            let mut out = vec![];
            b.iter(|| {
                let out = black_box(&mut out);
                out.clear();
                black_box(bincode::serialize_into(out, black_box(&v)).unwrap())
            })
        }

        #[bench]
        fn facet_xdr(b: &mut Bencher) {
            let v: $t = $b();
            b.iter(|| black_box(facet_xdr::to_vec(black_box(&v)).unwrap()))
        }
    }

    mod deserialize { use super::*;
        #[bench]
        fn facet_bitcode(b: &mut Bencher) {
            let original: $t = $b();
            let bytes = crate::serialize(&original);

            b.iter(|| {
                let deserialized: $t = crate::deserialize(black_box(bytes.as_slice())).unwrap();
                debug_assert_eq!(deserialized, original);
                deserialized
            })
        }

        #[bench]
        fn serde_bitcode(b: &mut Bencher) {
            let original: $t = $b();
            let bytes = bitcode::serialize(&original).unwrap();

            b.iter(|| {
                let deserialized: $t = bitcode::deserialize(black_box(bytes.as_slice())).unwrap();
                debug_assert_eq!(&*deserialized, &*original);
                deserialized
            })
        }

        #[bench]
        fn derive_bitcode(b: &mut Bencher) {
            let mut buffer = bitcode::Buffer::new();

            let original: $t = $b();
            let bytes = buffer.encode(&original).to_vec();

            b.iter(|| {
                let deserialized: $t = buffer.decode(black_box(bytes.as_slice())).unwrap();
                debug_assert_eq!(&*deserialized, &*original);
                deserialized
            })
        }

        #[bench]
        fn serde_bincode(b: &mut Bencher) {
            let original: $t = $b();
            let mut bytes = vec![];
            bincode::serialize_into(&mut bytes, &original).unwrap();

            b.iter(|| {
                let deserialized: $t = bincode::deserialize(black_box(bytes.as_slice())).unwrap();
                debug_assert_eq!(&*deserialized, &*original);
                deserialized
            })
        }

        #[bench]
        fn facet_xdr(b: &mut Bencher) {
            if std::any::type_name::<$t>() != "Mesh" {
                return; // Broken on benchmarks that have nested Vec because integers deserialize incorrectly.
            }

            let original: $t = $b();
            let bytes = facet_xdr::to_vec(&original).unwrap();

            b.iter(|| {
                let deserialized: $t = facet_xdr::deserialize(black_box(bytes.as_slice())).unwrap();
                // xdr current serializes u8 as u64 and deserializes it as u32, but perf should be similar.
                // debug_assert_eq!(&*deserialized, &*original);
                deserialized
            })
        }
    }
})+}}
bench!(mesh_one: Mesh, mesh_1k: Mesh, log_one: Log, log_1k: Log);
