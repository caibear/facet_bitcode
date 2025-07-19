use std::mem::MaybeUninit;

use crate::error::Error;
use crate::{consume::expect_eof, decoder::Decoder};
use facet_core::{Facet, Shape};

fn decoder(_shape: &'static Shape) -> Box<dyn Decoder> {
    todo!()
}

pub fn deserialize<'facet, T: Facet<'facet>>(mut bytes: &[u8]) -> Result<T, Error> {
    let decoder = decoder(T::SHAPE);
    let mut uninit = MaybeUninit::<T>::uninit();
    unsafe {
        decoder.decode_many(
            &mut bytes,
            std::ptr::slice_from_raw_parts_mut(uninit.as_mut_ptr() as *mut u8, 1),
        )?;
    }
    expect_eof(bytes)?;
    unsafe { Ok(uninit.assume_init()) }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use facet::Facet;

    fn roundtrip<'facet, T: Facet<'facet> + Debug + PartialEq>(t: &T) {
        let bytes = crate::serialize(t);
        let deserialized = crate::deserialize::<T>(&bytes).unwrap();
        assert_eq!(t, &deserialized);
    }

    #[test]
    #[should_panic = "not yet implemented"] // TODO implement.
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
}
