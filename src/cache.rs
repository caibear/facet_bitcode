use crate::codec::StaticCodec;
use facet_core::Shape;
use std::any::TypeId;

#[cfg(feature = "std")]
pub use fast_cache::codec_cached;
#[cfg(not(feature = "std"))]
pub use shared_cache::codec_cached;

#[cfg(feature = "std")]
mod fast_cache {
    use super::*;
    use crate::codec::_DUMMY_CODEC;
    use std::cell::Cell;

    struct _Dummy;
    thread_local! {
        static FAST_CACHE: Cell<(TypeId, StaticCodec)> = std::cell::Cell::new((TypeId::of::<_Dummy>(), _DUMMY_CODEC));
    }

    // Saves 3ns over shared_cache in benchmark with 0 contention.
    #[inline(always)]
    pub fn codec_cached(shape: &'static Shape) -> StaticCodec {
        let shape_id = shape.id.get();
        let (id, cached) = FAST_CACHE.get();
        if id == shape_id {
            cached
        } else {
            cache_miss(shape)
        }
    }

    #[cold]
    fn cache_miss(shape: &'static Shape) -> StaticCodec {
        let codec = super::shared_cache::codec_cached(shape);
        FAST_CACHE.set((shape.id.get(), codec));
        codec
    }
}

mod shared_cache {
    use super::*;
    use std::sync::{PoisonError, RwLock};

    static SHARED_CACHE: RwLock<Vec<(TypeId, StaticCodec)>> = RwLock::new(vec![]);

    pub fn codec_cached(shape: &'static Shape) -> StaticCodec {
        let shape_id = shape.id.get();
        if let Ok(codec) = entry_or_insert_index(
            &SHARED_CACHE.read().unwrap_or_else(PoisonError::into_inner),
            shape_id,
        ) {
            return codec;
        }

        let mut write_cache = SHARED_CACHE.write().unwrap_or_else(PoisonError::into_inner);
        match entry_or_insert_index(&write_cache, shape_id) {
            Ok(codec) => codec,
            Err(i) => {
                let codec = StaticCodec::new(shape);
                write_cache.insert(i, (shape_id, codec));
                codec
            }
        }
    }

    #[inline(never)]
    fn entry_or_insert_index(
        cache: &[(TypeId, StaticCodec)],
        shape_id: TypeId,
    ) -> Result<StaticCodec, usize> {
        cache
            .binary_search_by_key(&shape_id, |(id, _)| *id)
            .map(|i| cache[i].1)
    }
}
