use crate::codec::Codec;
use facet_core::Shape;
pub use fast::reflect;

type StaticCodec = &'static dyn Codec;

mod fast {
    use super::*;
    use crate::primitive::DUMMY_CODEC;
    use core::any::TypeId;
    use core::cell::Cell;

    struct _Dummy;
    thread_local! {
        static FAST_CACHE: Cell<(TypeId, StaticCodec)> = std::cell::Cell::new((TypeId::of::<_Dummy>(), &DUMMY_CODEC));
    }

    // Saves 3ns over shared cache in benchmark with 0 contention.
    #[inline(always)]
    pub fn reflect(shape: &'static Shape) -> StaticCodec {
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
        let codec = super::shared::reflect(shape);
        FAST_CACHE.set((shape.id.get(), codec));
        codec
    }
}

mod shared {
    use super::*;
    use core::any::TypeId;
    use std::sync::{PoisonError, RwLock};

    static SHARED_CACHE: RwLock<Vec<(TypeId, StaticCodec)>> = RwLock::new(vec![]);

    pub fn reflect(shape: &'static Shape) -> StaticCodec {
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
                let codec = Box::leak(crate::codec::reflect(shape));
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
