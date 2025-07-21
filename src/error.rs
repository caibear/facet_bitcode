use core::fmt::{Debug, Display, Formatter};

pub type Result<T> = core::result::Result<T, Error>;

/// Short version of `Err(error("..."))`.
pub fn err<T>(msg: &'static str) -> Result<T> {
    Err(error(msg))
}

/// Creates an error with a message that might be displayed.
pub fn error(_msg: &'static str) -> Error {
    #[cfg(debug_assertions)]
    return Error(_msg);
    #[cfg(not(debug_assertions))]
    Error(())
}

#[cfg(debug_assertions)]
type ErrorImpl = &'static str;
#[cfg(not(debug_assertions))]
type ErrorImpl = ();

/// Decoding / (De)serialization errors.
/// # Debug mode
/// In debug mode, the error contains a reason.
/// # Release mode
/// In release mode, the error is a zero-sized type for efficiency.
#[cfg_attr(test, derive(PartialEq))]
pub struct Error(ErrorImpl);
impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        #[cfg(debug_assertions)]
        return write!(f, "Error({:?})", self.0);
        #[cfg(not(debug_assertions))]
        f.write_str("Error(\"facet_bitcode error\")")
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        #[cfg(debug_assertions)]
        return f.write_str(self.0);
        #[cfg(not(debug_assertions))]
        f.write_str("facet_bitcode error")
    }
}
impl core::error::Error for Error {}
