use core::alloc::Layout;
use alloc::alloc;
use alloc::handle_alloc_error;
use core::ptr::NonNull;

// Based on https://github.com/rust-lang/rust/blob/6707bf0f59485cf054ac1095725df43220e4be20/library/alloc/src/raw_vec/mod.rs#L32C1-L38C2
enum AllocInit {
    /// The contents of the new memory are uninitialized.
    Uninitialized,
}

// Based on https://github.com/rust-lang/rust/blob/6707bf0f59485cf054ac1095725df43220e4be20/library/alloc/src/collections/mod.rs#L100-L121
#[allow(dead_code)]
enum TryReserveError {
    CapacityOverflow,
    AllocError {
        layout: Layout,
        non_exhaustive: (),
    },
}
use TryReserveError::*;

// Based on https://github.com/rust-lang/rust/blob/6707bf0f59485cf054ac1095725df43220e4be20/library/alloc/src/raw_vec/mod.rs#L86-L95
pub struct RawVecInner {
    pub ptr: NonNull<u8>, // std uses Unique<u8>
    pub cap: usize,       // std uses UsizeNoHighBit
}

impl RawVecInner {
    // Based on https://github.com/rust-lang/rust/blob/6707bf0f59485cf054ac1095725df43220e4be20/library/alloc/src/raw_vec/mod.rs#L149C1-L154C6
    #[must_use]
    #[inline]
    #[track_caller]
    pub fn with_capacity(capacity: usize, elem_layout: Layout) -> Self {
        match Self::try_allocate_in(capacity, AllocInit::Uninitialized, elem_layout) {
            Ok(res) => res,
            Err(err) => handle_error(err),
        }
    }

    // Based on https://github.com/rust-lang/rust/blob/6707bf0f59485cf054ac1095725df43220e4be20/library/alloc/src/raw_vec/mod.rs#L411C5-L416C6
    #[inline]
    const fn new_in(align: usize) -> Self {
        // Safety: Alignment -> usize type (caller uses layout.align() instead of layout.alignment(), both are nonzero).
        let ptr = unsafe { NonNull::new_unchecked(align as *mut u8) };
        // `cap: 0` means "unallocated". zero-sized types are ignored.
        Self { ptr, cap: 0 }
    }

    // Based on https://github.com/rust-lang/rust/blob/6707bf0f59485cf054ac1095725df43220e4be20/library/alloc/src/raw_vec/mod.rs#L453C5-L493C6
    fn try_allocate_in(
        capacity: usize,
        init: AllocInit,
        elem_layout: Layout,
    ) -> Result<Self, TryReserveError> {
        // We avoid `unwrap_or_else` here because it bloats the amount of
        // LLVM IR generated.
        let layout = match layout_array(capacity, elem_layout) {
            Ok(layout) => layout,
            Err(_) => return Err(CapacityOverflow.into()),
        };

        // Don't allocate here because `Drop` will not deallocate when `capacity` is 0.
        if layout.size() == 0 {
            return Ok(Self::new_in(elem_layout.align()));
        }

        if let Err(err) = alloc_guard(layout.size()) {
            return Err(err);
        }

        let result = match init {
            // Safety: layout was checked to have a nonzero size.
            AllocInit::Uninitialized => NonNull::new(unsafe { alloc::alloc(layout) }),
        };
        let ptr = match result {
            Some(ptr) => ptr,
            None => return Err(AllocError { layout, non_exhaustive: () }),
        };

        // Allocators currently return a `NonNull<[u8]>` whose length
        // matches the size requested. If that ever changes, the capacity
        // here should change to `ptr.len() / size_of::<T>()`.
        Ok(Self {
            ptr,
            cap: capacity,
        })
    }
}

// Based on https://github.com/rust-lang/rust/blob/6707bf0f59485cf054ac1095725df43220e4be20/library/alloc/src/raw_vec/mod.rs#L795-L800
#[cold]
#[track_caller]
fn handle_error(e: TryReserveError) -> ! {
    match e {
        CapacityOverflow => capacity_overflow(),
        AllocError { layout, .. } => handle_alloc_error(layout),
    }
}

// Based on https://github.com/rust-lang/rust/blob/6707bf0f59485cf054ac1095725df43220e4be20/library/alloc/src/raw_vec/mod.rs#L22C1-L30C2
#[inline(never)]
#[track_caller]
fn capacity_overflow() -> ! {
    panic!("capacity overflow");
}

// Exact copy of https://github.com/rust-lang/rust/blob/6707bf0f59485cf054ac1095725df43220e4be20/library/alloc/src/raw_vec/mod.rs#L810C1-L818C1
#[inline]
fn alloc_guard(alloc_size: usize) -> Result<(), TryReserveError> {
    if usize::BITS < 64 && alloc_size > isize::MAX as usize {
        Err(CapacityOverflow.into())
    } else {
        Ok(())
    }
}

// Exact copy of https://github.com/rust-lang/rust/blob/6707bf0f59485cf054ac1095725df43220e4be20/library/alloc/src/raw_vec/mod.rs#L819C1-L822C2
#[inline]
fn layout_array(cap: usize, elem_layout: Layout) -> Result<Layout, TryReserveError> {
    elem_layout.repeat(cap).map(|(layout, _pad)| layout).map_err(|_| CapacityOverflow.into())
}
