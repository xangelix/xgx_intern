pub mod float;

use std::{
    borrow::Cow,
    hash::{BuildHasher, Hash},
    marker::PhantomData,
};

use indexmap::IndexSet;

#[derive(Debug, thiserror::Error)]
pub enum InternerError {
    #[error("Interner handle space exhausted")]
    Overflow,
}

/// A generic, high-performance interner.
///
/// It stores each unique item only once and returns a lightweight index
/// handle for it. This is memory-efficient and allows for very fast
/// equality comparisons on the handles.
pub struct Interner<T, S, H = u32>
where
    T: Eq + Hash,
    S: BuildHasher,
    H: Copy + TryFrom<usize>, // for index -> handle
    usize: TryFrom<H>,        // for handle -> index
{
    items: IndexSet<T, S>,
    _handle: PhantomData<H>,
}

impl<T, S, H> Interner<T, S, H>
where
    T: Eq + Hash,
    S: BuildHasher,
    H: Copy + TryFrom<usize>,
    usize: TryFrom<H>,
{
    #[must_use]
    pub const fn new(hasher: S) -> Self {
        Self {
            items: IndexSet::with_hasher(hasher),
            _handle: PhantomData,
        }
    }

    /// Creates a new interner with a specified capacity.
    #[must_use]
    pub fn with_capacity(hasher: S, capacity: usize) -> Self {
        Self {
            items: IndexSet::with_capacity_and_hasher(capacity, hasher),
            _handle: PhantomData,
        }
    }

    // --- 1) Owned: no Clone bound needed
    pub fn intern_owned(&mut self, item: T) -> Result<H, InternerError> {
        let (idx, _inserted) = self.items.insert_full(item);
        Self::idx_to_handle(idx)
    }

    // --- 2) Borrowed ref: Clone only when we need to insert
    pub fn intern_ref(&mut self, item: &T) -> Result<H, InternerError>
    where
        T: Clone,
    {
        if let Some(idx) = self.items.get_index_of(item) {
            return Self::idx_to_handle(idx);
        }
        let h = Self::idx_to_handle(self.items.len())?;
        self.items.insert(item.clone());
        Ok(h)
    }

    // --- 3) Cow: Clone only if it's Borrowed
    pub fn intern_cow(&mut self, item: Cow<'_, T>) -> Result<H, InternerError>
    where
        T: Clone,
    {
        if let Some(idx) = self.items.get_index_of(item.as_ref()) {
            return Self::idx_to_handle(idx);
        }
        let h = Self::idx_to_handle(self.items.len())?;
        let owned = match item {
            Cow::Owned(o) => o,
            Cow::Borrowed(b) => b.clone(),
        };
        self.items.insert(owned);
        Ok(h)
    }

    #[inline]
    fn idx_to_handle(idx: usize) -> Result<H, InternerError> {
        H::try_from(idx).map_err(|_| InternerError::Overflow)
    }

    /// Resolves a handle back to &T. Returns None if the handle is out of range.
    #[must_use]
    #[inline]
    pub fn resolve(&self, handle: H) -> Option<&T> {
        let idx: usize = usize::try_from(handle).ok()?;
        self.items.get_index(idx)
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
