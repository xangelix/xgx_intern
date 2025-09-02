pub mod float;

use std::{
    borrow::Cow,
    hash::{BuildHasher, Hash},
};

use indexmap::IndexSet;

#[derive(Debug, thiserror::Error)]
pub enum InternerError {
    #[error("Interner overflowed u32 limit")]
    Overflow,
}

/// A generic, high-performance interner.
///
/// It stores each unique item only once and returns a lightweight `u32`
/// handle for it. This is memory-efficient and allows for very fast
/// equality comparisons on the handles.
pub struct Interner<T: Eq + Hash + Clone, S: BuildHasher> {
    items: IndexSet<T, S>,
}

impl<T: Eq + Hash + Clone, S: BuildHasher> Interner<T, S> {
    #[must_use]
    pub const fn new(hasher: S) -> Self {
        Self {
            items: IndexSet::with_hasher(hasher),
        }
    }

    /// Creates a new interner with a specified capacity.
    #[must_use]
    pub fn with_capacity(hasher: S, capacity: usize) -> Self {
        Self {
            items: IndexSet::with_capacity_and_hasher(capacity, hasher),
        }
    }

    /// Interns an item, accepting either a borrowed reference or an owned value.
    ///
    /// Returns a memory-efficient `u32` handle.
    ///
    /// Panics if the number of unique items exceeds `u32::MAX`.
    pub fn intern(&mut self, item: Cow<'_, T>) -> Result<u32, InternerError> {
        // First, perform a lookup using a borrowed reference.
        // `item.as_ref()` gives a `&T` regardless of the Cow's variant.
        if let Some(index) = self.items.get_index_of(item.as_ref()) {
            return Ok(index as u32);
        }

        // Before inserting a new item, check if we are at capacity.
        if self.items.len() == u32::MAX as usize {
            return Err(InternerError::Overflow);
        }

        // If the item is not found, insert it. `item.into_owned()` will
        // clone if borrowed or move if owned, avoiding a needless allocation.
        let index = self.items.insert_full(item.into_owned()).0;
        Ok(index as u32)
    }

    /// Resolves a `u32` handle back to the original item slice.
    ///
    /// Panics if the handle is invalid.
    #[must_use]
    #[inline]
    pub fn resolve(&self, handle: u32) -> Option<&T> {
        self.items.get_index(handle as usize)
    }

    /// Returns the number of unique items in the interner.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[must_use]
    pub fn export(self) -> Vec<T> {
        self.items.into_iter().collect()
    }
}
