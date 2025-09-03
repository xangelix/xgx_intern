/// Provides wrappers for interning floating-point types.
///
/// Standard `f32` and `f64` types do not implement `Eq` or `Hash` due to `NaN` semantics,
/// making them unusable with `Interner` directly. This module offers custom
/// types that provide a canonical representation for hashing and equality, allowing
/// floats to be reliably interned.
pub mod float;

use std::{
    borrow::Cow,
    hash::{BuildHasher, Hash},
    marker::PhantomData,
};

use indexmap::IndexSet;

/// Represents errors that can occur during an interning operation.
#[derive(Debug, thiserror::Error)]
pub enum InternerError {
    /// Occurs when the number of unique items exceeds the maximum value
    /// representable by the handle type `H`.
    ///
    /// For example, if the handle `H` is a `u32`, this error will be returned
    /// on the attempt to intern the 2^32-th unique item.
    #[error("Interner handle space exhausted")]
    Overflow,
}

/// A generic, high-performance interner for deduplicating values.
///
/// An interner stores each unique item only once and returns a lightweight, copyable
/// handle for it. Subsequent attempts to intern the same value will not store a new copy
/// but will return the handle to the existing item.
///
/// This data structure is highly memory-efficient when dealing with many duplicate
/// values (e.g., strings in a parser's AST). It also enables extremely fast
/// equality comparisons, as two handles are equal if and only if they represent the
/// same original value. This transforms potentially expensive deep comparisons
/// into simple integer comparisons.
///
/// # Type Parameters
///
/// - `T`: The type of the item to be interned. Must implement `Eq` and `Hash`.
/// - `S`: The `BuildHasher` used by the underlying map. This allows for swapping out
///   the hashing algorithm for performance-critical use cases (e.g., using `ahash` or `fxhash`).
/// - `H`: The handle type used to represent interned items. It defaults to `u32` but can
///   be customized (e.g., `u16` for memory savings if the number of unique items is low,
///   or `u64` if it is very high).
///
/// # Examples
///
/// ```
/// use std::collections::hash_map::RandomState;
///
/// use xgx_intern::{Interner, InternerError};
///
/// // Create an interner for strings.
/// let mut interner = Interner::<String, RandomState>::new(RandomState::new());
///
/// // Intern an owned string.
/// let handle1 = interner.intern_owned("hello".to_string()).unwrap();
///
/// // Intern a new string via a reference. This will clone the string and store it.
/// let new_string = "world".to_string();
/// let handle2 = interner.intern_ref(&new_string).unwrap();
///
/// // Interning an existing value (by reference) returns the original handle.
/// let existing_string = "hello".to_string();
/// let handle3 = interner.intern_ref(&existing_string).unwrap();
///
/// assert_eq!(handle1, handle3);
/// assert_ne!(handle1, handle2);
///
/// // We can resolve a handle back to the original value.
/// assert_eq!(interner.resolve(handle1), Some(&"hello".to_string()));
/// assert_eq!(interner.resolve(handle2), Some(&"world".to_string()));
///
/// // The interner only stores two unique strings.
/// assert_eq!(interner.len(), 2);
/// ```
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
    /// Creates a new, empty interner with the given `BuildHasher`.
    ///
    /// The `hasher` is used to construct the internal hash map. Different
    /// `BuildHasher` implementations can be used to optimize for speed or
    /// security. For example, `ahash` or `fxhash` can provide significant
    /// performance gains.
    #[must_use]
    pub const fn new(hasher: S) -> Self {
        Self {
            items: IndexSet::with_hasher(hasher),
            _handle: PhantomData,
        }
    }

    /// Creates a new interner with a specified capacity and `BuildHasher`.
    ///
    /// This is useful when the number of unique items to be interned is known
    /// in advance. Pre-allocating capacity can prevent multiple reallocations
    /// of the internal hash map, improving performance.
    #[must_use]
    pub fn with_capacity(hasher: S, capacity: usize) -> Self {
        Self {
            items: IndexSet::with_capacity_and_hasher(capacity, hasher),
            _handle: PhantomData,
        }
    }

    /// Interns an owned value, taking ownership.
    ///
    /// If the value already exists in the interner, its handle is returned.
    /// Otherwise, the value is stored and a new handle is created and returned.
    ///
    /// This is the most efficient method when you already have an owned value,
    /// as it avoids any potential clones.
    ///
    /// # Errors
    ///
    /// Returns `InternerError::Overflow` if the interner's handle capacity is exhausted.
    pub fn intern_owned(&mut self, item: T) -> Result<H, InternerError> {
        // Look up the item first. The `Borrow<T>` trait bound on `get_index_of`
        // allows us to look up an owned `T` using a reference.
        if let Some(idx) = self.items.get_index_of(&item) {
            return Self::idx_to_handle(idx);
        }

        // If the item is new, check for overflow *before* inserting to
        // maintain a consistent state if the operation fails.
        let handle = Self::idx_to_handle(self.items.len())?;
        self.items.insert(item);
        Ok(handle)
    }

    /// Interns a borrowed value by reference.
    ///
    /// If a value equal to `item` already exists in the interner, its handle is
    /// returned without any allocation. If the value is not present, `item` is
    /// cloned, the clone is stored, and a new handle is returned.
    ///
    /// This method requires `T: Clone` and is ideal for cases where you have a
    /// reference to a value and want to avoid cloning it if it's already been
    /// interned.
    ///
    /// # Errors
    ///
    /// Returns `InternerError::Overflow` if a new item is inserted and the
    /// interner's handle capacity is exhausted.
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

    /// Interns a value wrapped in a `Cow` (Clone-on-Write).
    ///
    /// This method provides a flexible interface that can accept either an owned
    /// or borrowed value.
    ///
    /// - If `item` is `Cow::Borrowed`, it behaves like `intern_ref`: the value is
    ///   cloned only if it's not already present in the interner.
    /// - If `item` is `Cow::Owned`, it behaves like `intern_owned`: the value is
    ///   moved into the interner, avoiding any clones.
    ///
    /// This method requires `T: Clone`.
    ///
    /// # Errors
    ///
    /// Returns `InternerError::Overflow` if a new item is inserted and the
    /// interner's handle capacity is exhausted.
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

    /// Internal helper to safely convert a `usize` index to a handle `H`.
    ///
    /// This is the single point of failure for handle space exhaustion.
    #[inline]
    fn idx_to_handle(idx: usize) -> Result<H, InternerError> {
        H::try_from(idx).map_err(|_| InternerError::Overflow)
    }

    /// Resolves a handle back to a reference to the interned value.
    ///
    /// Returns `Some(&T)` if the handle is valid and corresponds to a value in
    /// the interner. Returns `None` if the handle is invalid (e.g., out of bounds).
    #[must_use]
    #[inline]
    pub fn resolve(&self, handle: H) -> Option<&T> {
        let idx: usize = usize::try_from(handle).ok()?;
        self.items.get_index(idx)
    }

    /// Returns the number of unique items currently stored in the interner.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the interner contains no items.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Consumes the interner and returns a vector of all unique items.
    ///
    /// The items in the returned vector are ordered by their first insertion.
    /// The handle `H` for an item can be derived from its index in the vector,
    /// assuming no overflow has occurred.
    ///
    /// This can be useful for serialization or transferring the set of interned
    /// values to another context.
    #[must_use]
    pub fn export(self) -> Vec<T> {
        self.items.into_iter().collect()
    }
}
