use crate::error::{err, Result};

pub fn consume_byte_arrays<'a>(
    input: &mut &'a [u8],
    num_arrays: usize,
    array_length: usize,
) -> Result<&'a [u8]> {
    // Uses division to avoid the posibility of array_length * num_arrays overflowing.
    if input.len() / array_length < num_arrays {
        return err("EOF");
    }
    // Safety: Checked that num_arrays * array_length bytes exists above.
    unsafe {
        Ok(consume_byte_arrays_unchecked(
            input,
            num_arrays,
            array_length,
        ))
    }
}

/// Doesn't actually return arrays because use constants derived from generics to form types.
/// Safety: validate_byte_arrays must have succeded with the same parameters.
pub unsafe fn consume_byte_arrays_unchecked<'a>(
    input: &mut &'a [u8],
    num_arrays: usize,
    array_length: usize,
) -> &'a [u8] {
    let total_bytes = num_arrays.unchecked_mul(array_length);
    let (bytes, remaining) = input.split_at_unchecked(total_bytes);
    *input = remaining;
    bytes
}

/// Check if `input` is empty or return error.
pub fn expect_eof(input: &[u8]) -> Result<()> {
    #[allow(unexpected_cfgs)]
    if cfg!(not(fuzzing)) && !input.is_empty() {
        err("Expected EOF")
    } else {
        Ok(())
    }
}
