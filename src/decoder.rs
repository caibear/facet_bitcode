use crate::error::Result;

pub trait Decoder: Send + Sync {
    // TODO can this return a result or do we need something like bitcode populate?
    unsafe fn decode_many(&self, input: &mut &[u8], erased: *mut [u8]) -> Result<()>;
}
