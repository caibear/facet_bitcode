use crate::codec::DynamicCodec;
use crate::decoder::{decode_one_or_many, try_decode_in_place, Decoder};
use crate::encoder::{encode_one_or_many, try_encode_in_place, Encoder};
use crate::error::{err, error, Result};
use crate::primitive::PrimitiveCodec;
use crate::raw_vec_fork::RawVecInner;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::marker::PhantomData;
use core::mem::{ManuallyDrop, MaybeUninit};

type LengthInt = u32; // TODO usize or u64.

/// Types that can be converted to &[T] and from Box<[T]> in O(1).
trait BoxedSliceLike {
    /// Shouldn't implement drop.
    type ErasedOwned;

    /// Safety: `erased` must be valid to read one instance of [`Self::ErasedOwned`].
    unsafe fn as_erased_slice(erased: *const Self::ErasedOwned) -> *const [u8];

    /// Safety: `erased` must be valid to read one instance of [`Self::ErasedOwned`].
    unsafe fn as_erased_slice_mut(erased: *mut Self::ErasedOwned) -> *mut [u8];

    /// Safety: `erased` must be a valid boxed slice (with unknown type).
    unsafe fn from_erased_boxed_slice(erased: *mut [u8]) -> Self::ErasedOwned;
}

/// Indicates that the BoxedSliceCodec is for Box<[T]>.
pub struct BoxedSliceMarker;
impl BoxedSliceLike for BoxedSliceMarker {
    type ErasedOwned = *mut [u8];

    #[inline(always)]
    unsafe fn as_erased_slice(erased: *const Self::ErasedOwned) -> *const [u8] {
        // Safety: Caller guarentees that `erased` is valid to read.
        unsafe { *erased }.cast_const()
    }

    #[inline(always)]
    unsafe fn as_erased_slice_mut(erased: *mut Self::ErasedOwned) -> *mut [u8] {
        unsafe { *erased }
    }

    #[inline(always)]
    unsafe fn from_erased_boxed_slice(erased: *mut [u8]) -> Self::ErasedOwned {
        erased
    }
}

/// Indicates that the BoxedSliceCodec is for Vec<T>.
pub struct VecMarker;
impl BoxedSliceLike for VecMarker {
    // ManuallyDrop prevents calling invalid drop.
    // MaybeUninit helps against padding bytes in encode and fully uninit in decode.
    type ErasedOwned = ManuallyDrop<Vec<MaybeUninit<u8>>>;

    #[inline(always)]
    unsafe fn as_erased_slice(erased: *const Self::ErasedOwned) -> *const [u8] {
        // Safety: Caller guarentees that `erased` is valid to read.
        let uninit: *const [MaybeUninit<u8>] = unsafe { &*erased }.as_slice();
        uninit as *const [u8]
    }

    #[inline(always)]
    unsafe fn as_erased_slice_mut(erased: *mut Self::ErasedOwned) -> *mut [u8] {
        // Safety: Caller guarentees that `erased` is valid to read.
        let uninit: *mut [MaybeUninit<u8>] = unsafe { &mut *erased }.as_mut_slice();
        uninit as *mut [u8]
    }

    #[inline(always)]
    unsafe fn from_erased_boxed_slice(erased: *mut [u8]) -> Self::ErasedOwned {
        // TODO(safety) creates invalid Vecs for Vec<ZST>.
        ManuallyDrop::new(Vec::from_raw_parts(
            erased as *mut u8 as *mut MaybeUninit<u8>,
            erased.len(),
            erased.len(),
        ))
    }
}

pub struct BoxedSliceCodec<T> {
    lengths: PrimitiveCodec<LengthInt>,
    element_layout: Layout,
    elements: DynamicCodec,
    _spooky: PhantomData<fn(T)>,
}

impl<T> BoxedSliceCodec<T> {
    pub fn new(element_layout: Layout, elements: DynamicCodec) -> Self {
        Self {
            lengths: Default::default(),
            element_layout,
            elements,
            _spooky: PhantomData,
        }
    }
}

impl<T: BoxedSliceLike> Encoder for BoxedSliceCodec<T> {
    unsafe fn encode_one(&self, erased: *const u8, out: &mut Vec<u8>) {
        let slice = T::as_erased_slice(erased as *const T::ErasedOwned);
        let len = slice.len() as LengthInt;
        self.lengths
            .encode_one((&len) as *const LengthInt as *const u8, out);
        encode_one_or_many(&*self.elements, slice, out);
    }

    unsafe fn encode_many(&self, erased: *const [u8], out: &mut Vec<u8>) {
        // Using *const [u8] to represent type erased *const [T].
        #[allow(clippy::cast_slice_different_sizes)]
        let erased = erased as *const [T::ErasedOwned];

        let slices = (0..erased.len())
            .map(|i| unsafe { T::as_erased_slice((erased as *const T::ErasedOwned).add(i)) });
        let mut n_elements = 0;
        try_encode_in_place(
            &self.lengths,
            Layout::for_value(&(0 as LengthInt)),
            erased.len(),
            &mut |mut dst| {
                for slice in slices.clone() {
                    n_elements += slice.len();
                    std::ptr::write_unaligned(dst as *mut LengthInt, slice.len() as LengthInt);
                    dst = dst.byte_add(core::mem::size_of::<LengthInt>());
                }
            },
            out,
        );

        try_encode_in_place(
            &*self.elements,
            self.element_layout,
            n_elements,
            &mut |mut dst: *mut u8| {
                let element_size = self.element_layout.size();
                for slice in slices.clone() {
                    let slice_len_bytes = slice.len().unchecked_mul(element_size);
                    core::ptr::copy_nonoverlapping(slice as *const u8, dst, slice_len_bytes);
                    dst = dst.byte_add(slice_len_bytes);
                }
            },
            out,
        );
    }
}

impl<T: BoxedSliceLike> Decoder for BoxedSliceCodec<T> {
    fn validate(&self, input: &mut &[u8], length: usize) -> Result<()> {
        let before_lengths_consumed = *input;
        self.lengths.validate(input, length)?;
        // Safety: we validated that input contained enough bytes before
        // validate was called, and we use that slice, not the modified input.
        let iter = unsafe { self.lengths.iter(before_lengths_consumed, length) };

        if length > u32::MAX as usize {
            return err("length too large"); // TODO support usize length.
        }
        let mut sum = 0u64;
        for length in iter {
            let length: u32 = length; // If length is changed from u32, this needs to change to be safe.

            // Safety: we checked that there are no more than u32::MAX u32s, and u32::MAX * u32::MAX < u64::MAX.
            sum = unsafe { sum.unchecked_add(length as u64) };
        }
        let sum = sum.try_into().map_err(|_| error("length > usize::MAX"))?;

        self.elements.validate(input, sum)?;
        Ok(())
    }

    unsafe fn decode_one(&self, input: &mut &[u8], erased: *mut u8) {
        let mut length = MaybeUninit::<LengthInt>::uninit();
        self.lengths
            .decode_one(input, length.as_mut_ptr() as *mut u8);
        let length = length.assume_init() as usize;
        let erased_box = allocate_erased_box(length, self.element_layout);
        unsafe { *(erased as *mut T::ErasedOwned) = T::from_erased_boxed_slice(erased_box) };
        decode_one_or_many(&*self.elements, input, erased_box);
    }

    unsafe fn decode_many(&self, input: &mut &[u8], erased: *mut [u8]) {
        let erased = erased as *mut [T::ErasedOwned];

        let slices = (0..erased.len()).map(|i| unsafe { (erased as *mut T::ErasedOwned).add(i) });
        let mut n_elements = 0;
        try_decode_in_place(
            &self.lengths,
            Layout::for_value(&(0 as LengthInt)),
            erased.len(),
            &mut |mut src| {
                for slice in slices.clone() {
                    let length = std::ptr::read_unaligned(src as *const LengthInt) as usize;
                    src = src.byte_add(core::mem::size_of::<LengthInt>());
                    n_elements += length;
                    *slice = T::from_erased_boxed_slice(allocate_erased_box(
                        length,
                        self.element_layout,
                    ));
                }
            },
            input,
        );

        try_decode_in_place(
            &*self.elements,
            self.element_layout,
            n_elements,
            &mut |mut src| {
                let element_size = self.element_layout.size();
                for boxed_slice_like in slices.clone() {
                    let slice = T::as_erased_slice_mut(boxed_slice_like);
                    let slice_len_bytes = slice.len().unchecked_mul(element_size);
                    core::ptr::copy_nonoverlapping(src, slice as *mut u8, slice_len_bytes);
                    src = src.byte_add(slice_len_bytes);
                }
            },
            input,
        );
    }
}

#[inline]
fn allocate_erased_box(length: usize, element_layout: Layout) -> *mut [u8] {
    let erased_raw_vec = RawVecInner::with_capacity(length, element_layout);
    debug_assert_eq!(erased_raw_vec.cap, length); // Current implementation guarantees this.
    core::ptr::slice_from_raw_parts_mut(erased_raw_vec.ptr.as_ptr(), length)
}
