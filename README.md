
# [bitcode](https://crates.io/crates/bitcode) implementation for [facet](https://github.com/facet-rs/facet)

## Early Benchmarks

200-600 times faster than the fastest facet serializer facet_xdr ❗

5 times faster than the fastest serde serializer bincode ❗❗

1.1-2.5 times faster than the [#1 fastest rust serializer](https://github.com/djkoloski/rust_serialization_benchmark) bitcode derive ❗❓❗❓

```rust
Vertex {
    x: f32,
    y: f32,
    z: f32,
    r: u8,
    g: u8,
    b: u8,
}

vec![Vertex { .. }; 1000]
test benches::mesh_1k::deserialize::derive_bitcode        ... bench:       1,364.36 ns/iter (+/- 8.13)
test benches::mesh_1k::deserialize::facet_bitcode         ... bench:       1,246.39 ns/iter (+/- 9.39)
test benches::mesh_1k::deserialize::facet_xdr             ... bench:     239,599.73 ns/iter (+/- 2,684.90)
test benches::mesh_1k::deserialize::serde_bincode         ... bench:       6,389.17 ns/iter (+/- 29.99)
test benches::mesh_1k::deserialize::serde_bitcode         ... bench:      10,880.72 ns/iter (+/- 134.96)

test benches::mesh_1k::serialize::derive_bitcode          ... bench:       2,882.00 ns/iter (+/- 11.64)
test benches::mesh_1k::serialize::facet_bitcode           ... bench:       1,191.61 ns/iter (+/- 4.20)
test benches::mesh_1k::serialize::facet_xdr               ... bench:     665,900.70 ns/iter (+/- 8,286.51)
test benches::mesh_1k::serialize::serde_bincode           ... bench:       5,995.02 ns/iter (+/- 17.22)
test benches::mesh_1k::serialize::serde_bitcode           ... bench:       6,110.18 ns/iter (+/- 47.56)

vec![Vertex { .. }; 1]
test benches::mesh_one::deserialize::derive_bitcode       ... bench:          25.94 ns/iter (+/- 0.19)
test benches::mesh_one::deserialize::facet_bitcode        ... bench:          32.47 ns/iter (+/- 0.70)
test benches::mesh_one::deserialize::facet_xdr            ... bench:         414.10 ns/iter (+/- 13.07)
test benches::mesh_one::deserialize::serde_bincode        ... bench:          16.67 ns/iter (+/- 0.18)
test benches::mesh_one::deserialize::serde_bitcode        ... bench:         133.14 ns/iter (+/- 3.89)

test benches::mesh_one::serialize::derive_bitcode         ... bench:          37.81 ns/iter (+/- 1.54)
test benches::mesh_one::serialize::facet_bitcode          ... bench:          13.75 ns/iter (+/- 0.08)
test benches::mesh_one::serialize::facet_xdr              ... bench:         813.32 ns/iter (+/- 11.59)
test benches::mesh_one::serialize::serde_bincode          ... bench:           5.88 ns/iter (+/- 0.15)
test benches::mesh_one::serialize::serde_bitcode          ... bench:         268.27 ns/iter (+/- 4.08)
```

## TODO
- [ ] Length > u32::MAX
- [ ] swap bytes of integers on big endian

### Types
- [x] u64, u32, u16...
- [x] Box<[T]> (hack since no impl facet::Shape for Box<[T]> yet)
- [x] Structs
- [x] Vec<T>
- [ ] String
- [ ] str
- [ ] [T; N]
- [ ] Option
- [ ] Enums
- [ ] usize/isize
- [ ] Fallback for opaque types

### Large Input Optimizations
- [ ] AOT optimizer
    - [ ] flatten StructCodecs
    - [ ] reform new StructCodecs when element_size/stride is too large
- [ ] scratch allocator
- [ ] rayon (unlike most serializers everything is trivially parallelizable)
    - [ ] par_iter on byte copying loops
    - [ ] par_iter on struct field loop
- [ ] JIT optimizer
    - [ ] ??? Profit

### Small Input Optimizations
- [ ] function ptrs instead of &dyn Codec
- [ ] slice iterator instead of slice to validate/decode

### Size Optimizations (from bitcode)
- [ ] bool -> 1 bit
- [ ] u64 -> u32 -> u16 -> u8
- [ ] u8 -> u4 -> u2 -> u1
