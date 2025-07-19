use crate::encoder::Encoder;
use facet_core::Shape;
use std::any::TypeId;

#[cfg(feature = "std")]
pub use fast_cache::encoder_cached;
#[cfg(not(feature = "std"))]
pub use shared_cache::encoder_cached;

#[cfg(feature = "std")]
mod fast_cache {
    use super::*;
    use crate::primitive::_DUMMY_ENCODER;
    use std::cell::Cell;

    struct _Dummy;
    thread_local! {
        static FAST_ENCODER_CACHE: Cell<(TypeId, &'static dyn Encoder)> = std::cell::Cell::new((TypeId::of::<_Dummy>(), &_DUMMY_ENCODER));
    }

    // Saves 3ns over shared_cache in benchmark with 0 contention.
    #[inline(always)]
    pub fn encoder_cached(shape: &'static Shape) -> &'static dyn Encoder {
        let shape_id = shape.id.get();
        let (id, cached) = FAST_ENCODER_CACHE.get();
        if id == shape_id {
            cached
        } else {
            cache_miss(shape)
        }
    }

    #[cold]
    fn cache_miss(shape: &'static Shape) -> &'static dyn Encoder {
        let encoder = super::shared_cache::encoder_cached(shape);
        FAST_ENCODER_CACHE.set((shape.id.get(), encoder));
        encoder
    }
}

mod shared_cache {
    use super::*;
    use crate::serialize::encoder;
    use std::sync::RwLock;

    static ENCODER_CACHE: RwLock<Vec<(TypeId, &'static dyn Encoder)>> = RwLock::new(vec![]);

    pub fn encoder_cached(shape: &'static Shape) -> &'static dyn Encoder {
        let shape_id = shape.id.get();
        if let Ok(encoder) = entry_or_insert_index(&ENCODER_CACHE.read().unwrap(), shape_id) {
            return encoder;
        }

        let mut write_cache = ENCODER_CACHE.write().unwrap();
        match entry_or_insert_index(&write_cache, shape_id) {
            Ok(encoder) => encoder,
            Err(i) => {
                let encoder = Box::leak(encoder(shape));
                write_cache.insert(i, (shape_id, encoder));
                encoder
            }
        }
    }

    #[inline(never)]
    fn entry_or_insert_index(
        cache: &[(TypeId, &'static dyn Encoder)],
        shape_id: TypeId,
    ) -> Result<&'static dyn Encoder, usize> {
        cache
            .binary_search_by_key(&shape_id, |(id, _)| *id)
            .map(|i| cache[i].1)
    }
}
