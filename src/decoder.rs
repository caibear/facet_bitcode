use crate::error::Result;

pub trait Decoder: Send + Sync {
    /// Validates that enough bytes are present and that they
    /// don't contain invalid values for e.g. bool or char.
    ///
    /// needs to happen before decoding for two reasons:
    /// 1. so we don't allocate memory for elements that don't exist
    /// 2. so we don't have to implement dropping a partially initalized output
    fn validate(&self, input: &mut &[u8], length: usize) -> Result<()>;

    /// Required to have the exact same results (but possibly faster) as
    /// `unsafe { decoder.decode_many(input, std::ptr::slice_from_raw_parts_mut(erased, 1)) };`
    /// Safety: ^^^
    unsafe fn decode_one(&self, input: &mut &[u8], erased: *mut u8);

    /// TODO(optimization) use a structure that can avoid mutating length such as a slice iterator or a pointer.
    /// Safety: validate must have succeded with the same parameters.
    unsafe fn decode_many(&self, input: &mut &[u8], erased: *mut [u8]);
}
