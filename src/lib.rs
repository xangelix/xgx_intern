use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
};

use ahash::RandomState;
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
pub struct Interner<T: Eq + Hash + Clone> {
    items: IndexSet<T, RandomState>,
}

impl<T: Eq + Hash + Clone> Default for Interner<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Eq + Hash + Clone> Interner<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: IndexSet::with_hasher(RandomState::new()),
        }
    }

    /// Creates a new interner with a specified capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: IndexSet::with_capacity_and_hasher(capacity, RandomState::new()),
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

// A wrapper around f64 that implements Eq and Hash based on bit patterns.
#[derive(Clone, Copy, PartialOrd)]
pub struct HashableF64(pub f64);

impl PartialEq for HashableF64 {
    fn eq(&self, other: &Self) -> bool {
        // Two floats are equal if and only if their bit patterns are identical.
        // This means 0.0 and -0.0 are treated as different, and NaN == NaN.
        self.0.to_bits() == other.0.to_bits()
    }
}

// Since we've defined a total equality relation, we can implement Eq.
impl Eq for HashableF64 {}

impl Hash for HashableF64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the underlying bits of the float.
        self.0.to_bits().hash(state);
    }
}

impl From<HashableF64> for f64 {
    fn from(value: HashableF64) -> Self {
        value.0
    }
}

impl From<f64> for HashableF64 {
    fn from(value: f64) -> Self {
        Self(value)
    }
}
