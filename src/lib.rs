#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![no_std]

/// Provides wrappers for interning floating-point types.
///
/// Standard `f32` and `f64` types do not implement `Eq` or `Hash` due to `NaN` semantics,
/// making them unusable with `Interner` directly. This module offers custom
/// types that provide a canonical representation for hashing and equality, allowing
/// floats to be reliably interned.
pub mod float;

/// Provides the `FromRef` trait for constructing owned types from references.
pub mod from_ref;

pub use float::{HashableF32, HashableF64};
pub use from_ref::FromRef;

extern crate alloc;

use alloc::{
    borrow::{Cow, ToOwned},
    string::String,
    vec::Vec,
};
use core::{
    borrow::Borrow,
    fmt,
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

impl<T, S, H> Default for Interner<T, S, H>
where
    T: Eq + Hash,
    S: BuildHasher + Default,
    H: Copy + TryFrom<usize>,
    usize: TryFrom<H>,
{
    #[inline]
    fn default() -> Self {
        Self::new(S::default())
    }
}

impl<T, S, H> fmt::Debug for Interner<T, S, H>
where
    T: Eq + Hash,
    S: BuildHasher,
    H: Copy + TryFrom<usize>,
    usize: TryFrom<H>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Interner")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .finish()
    }
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
    /// This method requires `T: FromRef` and is ideal for cases where you have
    /// a reference to a value or slice and want to avoid cloning or boxing if
    /// it's already been interned.
    ///
    /// # Errors
    ///
    /// Returns `InternerError::Overflow` if a new item is inserted and the
    /// interner's handle capacity is exhausted.
    pub fn intern_ref<Q>(&mut self, item: &Q) -> Result<H, InternerError>
    where
        T: Borrow<Q> + FromRef<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(idx) = self.items.get_index_of(item) {
            return Self::idx_to_handle(idx);
        }
        let h = Self::idx_to_handle(self.items.len())?;
        self.items.insert(T::from_ref(item));
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
    pub fn intern_cow<Q>(&mut self, item: Cow<'_, Q>) -> Result<H, InternerError>
    where
        T: Borrow<Q> + Clone,
        Q: ToOwned<Owned = T> + Hash + Eq + ?Sized,
    {
        if let Some(idx) = self.items.get_index_of(item.as_ref()) {
            return Self::idx_to_handle(idx);
        }
        let h = Self::idx_to_handle(self.items.len())?;
        self.items.insert(item.into_owned());
        Ok(h)
    }

    /// Returns the existing handle for `key` or inserts a newly constructed value.
    pub fn intern_ref_or_insert_with<Q, F>(&mut self, key: &Q, make: F) -> Result<H, InternerError>
    where
        T: Borrow<Q> + Clone,
        Q: Hash + Eq + ?Sized,
        F: FnOnce() -> T,
    {
        if let Some(idx) = self.items.get_index_of(key) {
            return Self::idx_to_handle(idx);
        }
        let h = Self::idx_to_handle(self.items.len())?;
        self.items.insert(make());
        Ok(h)
    }

    /// Returns the handle for `item` if present, without inserting or cloning.
    #[inline]
    pub fn lookup_handle<Q>(&self, item: &Q) -> Result<Option<H>, InternerError>
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.items
            .get_index_of(item)
            .map_or(Ok(None), |idx| Ok(Some(Self::idx_to_handle(idx)?)))
    }

    /// Returns true if an equal item is present.
    #[inline]
    pub fn contains<Q>(&self, item: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.items.contains(item)
    }

    /// Removes a value from the interner and returns the Handle and the Value.
    ///
    /// # ⚠️ Performance Warning: O(n)
    ///
    /// Unlike standard `HashMap` removal which is O(1), this operation is **O(n)**
    /// (linear time) because the interner is backed by a contiguous vector to
    /// preserve ordering.
    ///
    /// When an item is removed, all subsequent items must be **shifted to the left**
    /// to fill the gap.
    ///
    /// # ⚠️ Handle Invalidation
    ///
    /// Because indices shift, **handles for items inserted after this one will change**.
    ///
    /// ## Example Scenario
    ///
    /// Imagine an interner with items `[A, B, C]` corresponding to handles `0, 1, 2`.
    ///
    /// 1. You remove `B` (handle `1`).
    /// 2. `C` shifts left to fill the gap.
    /// 3. The storage is now `[A, C]`.
    ///
    /// **The Consequence:**
    /// * Handle `0` (`A`) remains valid.
    /// * Handle `2` (which used to be `C`) is now out of bounds!
    /// * Handle `1` (which used to be `B`) now resolves to `C`.
    ///
    /// # Handle Recovery
    ///
    /// Since the shift is deterministic, you can "repair" your existing handles
    /// if you are tracking them.
    ///
    /// * **Handles < removed:** Unaffected.
    /// * **Handles > removed:** Must be decremented by 1.
    ///
    /// ```text
    /// if my_handle > removed_handle {
    ///     my_handle -= 1;
    /// }
    /// ```
    /// See [`repair_handles`](Self::repair_handles) for a helper that automates this.
    pub fn remove<Q>(&mut self, item: &Q) -> Option<(H, T)>
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        // shift_remove_full returns (index, value)
        // We use shift_remove to preserve the relative order of remaining items.
        let (idx, val) = self.items.shift_remove_full(item)?;

        // The index returned by IndexSet is guaranteed to fit in usize.
        // We convert it back to H to return to the user.
        // We suppress the error here because if it was in the map, it had a valid handle.
        let handle = H::try_from(idx).ok()?;

        Some((handle, val))
    }

    /// Removes the item associated with the given `handle`.
    ///
    /// # Returns
    ///
    /// - `Some(T)`: The value that was removed, if the handle was valid.
    /// - `None`: If the handle was invalid (e.g. out of bounds).
    ///
    /// # ⚠️ Performance & Invalidation
    ///
    /// Like [`remove`](Self::remove), this operation is **O(n)** and will shift
    /// the indices of all subsequent items.
    ///
    /// Any existing handle `h` where `h > handle` must be decremented by 1 to
    /// remain valid.
    /// See [`repair_handles`](Self::repair_handles) for a helper that automates this.
    pub fn remove_handle(&mut self, handle: H) -> Option<T> {
        let idx = usize::try_from(handle).ok()?;
        self.items.shift_remove_index(idx)
    }

    /// A helper to update a collection of handles after a removal.
    ///
    /// When you call `remove`, handles greater than the removed index become invalid.
    /// This helper iterates over your collection of handles and decrements those
    /// that need to shift down, restoring their validity.
    ///
    /// # Generic Support
    ///
    /// This accepts any iterator that yields `&mut H`. This means it works with:
    /// - `&mut [H]` (slices and vectors of handles)
    /// - `.iter_mut()` on custom collections
    /// - `.iter_mut().map(|item| &mut item.id)` for structs containing handles
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (removed_h, _) = interner.remove("ItemB").unwrap();
    ///
    /// // Fix a simple vector of handles
    /// interner.repair_handles(removed_h, &mut my_handle_vec);
    ///
    /// // Fix handles inside a custom struct
    /// interner.repair_handles(removed_h, my_structs.iter_mut().map(|s| &mut s.handle));
    /// ```
    pub fn repair_handles<'a, I>(&self, removed: H, handles: I)
    where
        I: IntoIterator<Item = &'a mut H>,
        H: 'a + PartialOrd,
    {
        for h in handles {
            if *h > removed {
                // We rely on the generic H <-> usize conversion to perform the decrement.
                // We can safely unwrap here because:
                // 1. If h > removed, h must be >= 1.
                // 2. h - 1 is guaranteed to be a valid index that previously existed.
                if let Ok(idx) = usize::try_from(*h)
                    && let Ok(shifted) = H::try_from(idx - 1)
                {
                    *h = shifted;
                }
            }
        }
    }

    /// Current capacity, in number of items.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.items.capacity()
    }

    /// Reserves capacity for at least `additional` more items.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.items.reserve(additional);
    }

    /// Shrinks capacity to fit the current length.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.items.shrink_to_fit();
    }

    /// Removes all items.
    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
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

    /// Iterates over all unique items in insertion order.
    ///
    /// Note: `&Interner` also implements `IntoIterator`, so you can write:
    /// `for item in &interner { /* item: &T */ }`
    #[inline]
    pub fn iter(&self) -> indexmap::set::Iter<'_, T> {
        self.items.iter()
    }

    /// Consumes the interner and returns a vector of all unique items.
    ///
    /// The items in the returned vector are ordered by their first insertion.
    /// The handle `H` for an item can be derived from its index in the vector,
    /// assuming no overflow has occurred.
    ///
    /// This can be useful for serialization or transferring the set of interned
    /// values to another context.
    #[doc(alias = "into_vec")]
    #[must_use]
    pub fn export(self) -> Vec<T> {
        self.items.into_iter().collect()
    }
}

impl<'a, T, S, H> IntoIterator for &'a Interner<T, S, H>
where
    T: Eq + Hash,
    S: BuildHasher,
    H: Copy + TryFrom<usize>,
    usize: TryFrom<H>,
{
    type Item = &'a T;
    type IntoIter = indexmap::set::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<T, S, H> IntoIterator for Interner<T, S, H>
where
    T: Eq + Hash,
    S: BuildHasher,
    H: Copy + TryFrom<usize>,
    usize: TryFrom<H>,
{
    type Item = T;
    type IntoIter = indexmap::set::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<T, S, H> Interner<T, S, H>
where
    T: Eq + Hash + AsRef<str>,
    S: BuildHasher,
    H: Copy + TryFrom<usize>,
    usize: TryFrom<H>,
{
    /// Consumes the interner and flattens all strings into a single contiguous arena.
    ///
    /// This returns a tuple containing:
    /// 1. `String`: A massive string containing all interned values concatenated together.
    /// 2. `Vec<usize>`: A list of offsets.
    ///
    /// # How to use
    ///
    /// The string associated with handle `h` is located at:
    /// `&arena[offsets[h] .. offsets[h+1]]`
    ///
    /// # Efficiency
    ///
    /// This is much more memory efficient than `export()` for large numbers of small strings,
    /// as it removes the overhead of `String` structs (24 bytes) and heap allocators (16+ bytes)
    /// per item.
    pub fn export_arena(self) -> (String, Vec<usize>) {
        // 1. Calculate total bytes needed to perform exactly ONE allocation.
        // We iterate once to count. This is cheap (RAM access).
        let total_bytes: usize = self.items.iter().map(|s| s.as_ref().len()).sum();
        let count = self.items.len();

        // 2. Allocate the arena and the offsets table.
        let mut arena = String::with_capacity(total_bytes);
        let mut offsets = Vec::with_capacity(count + 1);

        // 3. The first offset is always 0.
        offsets.push(0);

        // 4. Fill the arena.
        // IndexSet iteration preserves insertion order, so handle IDs remain valid.
        for item in self.items {
            arena.push_str(item.as_ref());
            offsets.push(arena.len());
        }

        (arena, offsets)
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        borrow::Cow,
        boxed::Box,
        rc::Rc,
        string::{String, ToString as _},
        sync::Arc,
        vec::Vec,
    };
    use core::hash::BuildHasherDefault;

    use ahash::RandomState;
    use rustc_hash::FxHasher;

    use super::{Interner, InternerError};

    // A helper to create a standard interner for tests that use strings.
    fn create_string_interner() -> Interner<String, RandomState> {
        Interner::new(RandomState::new())
    }

    #[test]
    fn test_new_and_empty() {
        let interner = create_string_interner();
        assert!(interner.is_empty());
        assert_eq!(interner.len(), 0);
    }

    #[test]
    fn test_intern_owned_and_resolve() {
        let mut interner = create_string_interner();
        let item = "hello".to_string();
        let handle = interner.intern_owned(item.clone()).unwrap();

        assert!(!interner.is_empty());
        assert_eq!(interner.len(), 1);
        assert_eq!(interner.resolve(handle), Some(&item));
    }

    #[test]
    fn test_intern_owned_duplicate_returns_same_handle() {
        let mut interner = create_string_interner();
        let item1 = "hello".to_string();
        let item2 = "hello".to_string();

        let handle1 = interner.intern_owned(item1).unwrap();
        let handle2 = interner.intern_owned(item2).unwrap();

        assert_eq!(handle1, handle2);
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn test_intern_ref_and_resolve() {
        let mut interner = create_string_interner();
        let item = "world".to_string();

        let handle = interner.intern_ref(&item).unwrap();
        assert_eq!(interner.len(), 1);
        assert_eq!(interner.resolve(handle), Some(&item));
    }

    #[test]
    fn test_intern_ref_and_resolve_box_str() {
        let mut interner = Interner::<Box<str>, RandomState>::new(RandomState::new());
        let item = "world";

        let handle = interner.intern_ref(item).unwrap();
        assert_eq!(interner.len(), 1);
        assert_eq!(interner.resolve(handle).map(|s| &**s), Some(item));
    }

    #[test]
    fn test_intern_ref_and_resolve_rc_str() {
        let mut interner = Interner::<Rc<str>, RandomState>::new(RandomState::new());
        let item = "world";

        let handle = interner.intern_ref(item).unwrap();
        assert_eq!(interner.len(), 1);
        assert_eq!(interner.resolve(handle).map(|s| &**s), Some(item));
    }

    #[test]
    fn test_intern_ref_and_resolve_arc_str() {
        let mut interner = Interner::<Arc<str>, RandomState>::new(RandomState::new());
        let item = "world";

        let handle = interner.intern_ref(item).unwrap();
        assert_eq!(interner.len(), 1);
        assert_eq!(interner.resolve(handle).map(|s| &**s), Some(item));
    }

    #[test]
    fn test_intern_ref_and_resolve_vec_u8() {
        let mut interner = Interner::<Vec<u8>, RandomState>::new(RandomState::new());
        let item = "world";

        let handle = interner.intern_ref(item.as_bytes()).unwrap();
        assert_eq!(interner.len(), 1);
        assert_eq!(
            interner.resolve(handle).map(alloc::vec::Vec::as_slice),
            Some(item.as_bytes()),
        );
    }

    #[test]
    fn test_intern_ref_duplicate_returns_same_handle() {
        let mut interner = create_string_interner();
        let item = "world".to_string();

        let handle_owned = interner.intern_owned(item.clone()).unwrap();
        assert_eq!(interner.len(), 1);

        let handle_ref = interner.intern_ref(&item).unwrap();
        assert_eq!(handle_owned, handle_ref);
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn test_intern_cow_variants() {
        let mut interner = create_string_interner();
        let item = "cow".to_string();

        // Intern using Cow::Owned. We must specify the type for the Cow's generic
        // parameter to resolve the ambiguity between `String` and `str`.
        let handle1 = interner
            .intern_cow(Cow::<String>::Owned(item.clone()))
            .unwrap();
        assert_eq!(interner.len(), 1);
        assert_eq!(interner.resolve(handle1), Some(&item));

        // Intern using Cow::Borrowed, which should find the existing entry
        let handle2 = interner.intern_cow(Cow::Borrowed(&item)).unwrap();
        assert_eq!(handle1, handle2);
        assert_eq!(interner.len(), 1);

        // Intern a new item via Cow::Borrowed
        let new_item = "new_cow".to_string();
        let handle3 = interner.intern_cow(Cow::Borrowed(&new_item)).unwrap();
        assert_ne!(handle1, handle3);
        assert_eq!(interner.len(), 2);
        assert_eq!(interner.resolve(handle3), Some(&new_item));
    }

    #[test]
    fn test_mixed_interning_provides_consistent_handles() {
        let mut interner = create_string_interner();
        let val = "test".to_string();

        let h_owned = interner.intern_owned(val.clone()).unwrap();
        let h_ref = interner.intern_ref(&val).unwrap();
        let h_cow = interner.intern_cow(Cow::Borrowed(&val)).unwrap();

        assert_eq!(h_owned, h_ref);
        assert_eq!(h_ref, h_cow);
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn test_resolve_invalid_handle_returns_none() {
        let interner = create_string_interner();
        // Create an out-of-bounds handle. u32 is the default.
        let invalid_handle: u32 = 999;
        assert_eq!(interner.resolve(invalid_handle), None);
    }

    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    struct TestStruct {
        id: u32,
        name: String,
    }

    #[test]
    fn test_with_custom_struct_type() {
        let mut interner: Interner<TestStruct, RandomState> = Interner::new(RandomState::new());
        let item1 = TestStruct {
            id: 1,
            name: "one".into(),
        };
        let item2 = TestStruct {
            id: 1,
            name: "one".into(),
        };
        let item3 = TestStruct {
            id: 2,
            name: "two".into(),
        };

        let h1 = interner.intern_ref(&item1).unwrap();
        let h2 = interner.intern_ref(&item2).unwrap();
        let h3 = interner.intern_ref(&item3).unwrap();

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert_eq!(interner.len(), 2);
        assert_eq!(interner.resolve(h1), Some(&item1));
    }

    #[test]
    fn test_custom_handle_type_u16() {
        let mut interner: Interner<i32, RandomState, u16> = Interner::new(RandomState::new());
        let h1 = interner.intern_owned(100).unwrap();
        let h2 = interner.intern_owned(200).unwrap();
        let h3 = interner.intern_owned(100).unwrap();

        assert_eq!(h1, 0u16);
        assert_eq!(h2, 1u16);
        assert_eq!(h1, h3);
        assert_eq!(interner.len(), 2);
    }

    #[test]
    fn test_handle_overflow_error() {
        // Use a small handle type (u8) to make overflow easy to test.
        let mut interner: Interner<u16, RandomState, u8> = Interner::new(RandomState::new());

        // Intern 256 unique values (0 to 255), which should succeed.
        for i in 0..=255 {
            let handle_res = interner.intern_owned(i as u16);
            assert!(handle_res.is_ok());
            assert_eq!(handle_res.unwrap(), i as u8);
        }
        assert_eq!(interner.len(), 256);

        // The next unique insertion (the 257th) should fail.
        let overflow_res = interner.intern_owned(256);
        assert!(matches!(overflow_res, Err(InternerError::Overflow)));

        // The length should not have changed after the failed insertion.
        assert_eq!(interner.len(), 256);
    }

    #[test]
    fn test_custom_hasher_fxhash() {
        // Use FxHasher for potentially faster hashing of integers.
        type FxBuildHasher = BuildHasherDefault<FxHasher>;
        let mut interner: Interner<i64, FxBuildHasher> = Interner::new(FxBuildHasher::default());

        let h1 = interner.intern_owned(12345).unwrap();
        let h2 = interner.intern_owned(12345).unwrap();

        assert_eq!(h1, h2);
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn test_export_preserves_insertion_order() {
        let mut interner = create_string_interner();
        let h1 = interner.intern_owned("first".to_string()).unwrap();
        let h2 = interner.intern_owned("second".to_string()).unwrap();
        let _ = interner.intern_owned("first".to_string()).unwrap(); // Duplicate, should not affect order.

        let exported_data = interner.export();

        let expected = alloc::vec!["first".to_string(), "second".to_string()];
        assert_eq!(exported_data, expected);

        // The index from the exported vec should correspond to the handle.
        let idx1: usize = h1.try_into().ok().unwrap();
        let idx2: usize = h2.try_into().ok().unwrap();
        assert_eq!(exported_data[idx1], "first");
        assert_eq!(exported_data[idx2], "second");
    }

    #[test]
    fn test_into_iterator_ref() {
        let mut interner = create_string_interner();
        interner.intern_ref("a").unwrap();
        interner.intern_ref("b").unwrap();

        let mut collected = Vec::new();
        for s in &interner {
            collected.push(s.as_str());
        }

        assert_eq!(collected, alloc::vec!["a", "b"]);
    }

    #[test]
    fn test_get_does_not_insert() {
        let mut interner = create_string_interner();
        assert!(interner.lookup_handle("x").is_ok_and(|h| h.is_none()));
        assert!(interner.is_empty());

        let h = interner.intern_ref("x").unwrap();
        assert_eq!(interner.lookup_handle("x").unwrap(), Some(h));
        assert_eq!(interner.len(), 1);
    }

    #[test]
    fn test_contains() {
        let mut interner = create_string_interner();
        interner.intern_ref("abc").unwrap();
        assert!(interner.contains("abc"));
        assert!(!interner.contains("def"));
    }

    #[test]
    fn test_interner_utilities() {
        let mut interner = Interner::<String, RandomState>::with_capacity(RandomState::new(), 10);

        // Test Capacity
        assert!(interner.capacity() >= 10);

        interner.intern_ref("a").unwrap();
        interner.intern_ref("b").unwrap();

        // Test Reserve
        interner.reserve(100);
        assert!(interner.capacity() >= 102);

        // Test Shrink
        interner.shrink_to_fit();
        assert!(interner.capacity() >= 2);

        // Test Debug formatting
        let debug_str = alloc::format!("{interner:?}");
        assert!(debug_str.contains("Interner"));
        assert!(debug_str.contains("len: 2"));

        // Test Clear
        interner.clear();
        assert!(interner.is_empty());
        assert_eq!(interner.len(), 0);
    }

    #[test]
    fn test_export_arena() {
        let mut interner = create_string_interner();
        let h1 = interner.intern_ref("hello").unwrap();
        let h2 = interner.intern_ref("world").unwrap();

        let (arena, offsets) = interner.export_arena();

        assert_eq!(arena, "helloworld");
        assert_eq!(offsets, alloc::vec![0, 5, 10]);

        // Validate manual reconstruction
        let idx1: usize = h1.try_into().unwrap();
        let s1 = &arena[offsets[idx1]..offsets[idx1 + 1]];
        assert_eq!(s1, "hello");

        let idx2: usize = h2.try_into().unwrap();
        let s2 = &arena[offsets[idx2]..offsets[idx2 + 1]];
        assert_eq!(s2, "world");
    }

    #[test]
    fn test_intern_ref_or_insert_with() {
        let mut interner = create_string_interner();

        // 1. Insert new via closure
        let h1 = interner
            .intern_ref_or_insert_with("key", || "key_computed".to_string())
            .unwrap();
        assert_eq!(interner.resolve(h1), Some(&"key_computed".to_string()));

        // 2. Lookup existing (closure should NOT run)
        let mut called = false;
        let h2 = interner
            .intern_ref_or_insert_with("key_computed", || {
                called = true;
                "should_not_exist".to_string()
            })
            .unwrap();

        assert_eq!(h1, h2);
        assert!(!called, "Closure should not be called if item exists");
    }

    #[test]
    fn test_error_display() {
        let err = InternerError::Overflow;
        assert_eq!(alloc::format!("{err}"), "Interner handle space exhausted");
    }

    #[test]
    fn test_into_iterator_owned() {
        let mut interner = create_string_interner();
        interner.intern_ref("a").unwrap();
        interner.intern_ref("b").unwrap();

        // This consumes the interner
        let vec: Vec<String> = interner.into_iter().collect();
        // Sort to ensure deterministic comparison, though IndexSet preserves insertion order
        // so it should be ["a", "b"]
        assert_eq!(vec, alloc::vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_export_arena_empty() {
        let interner = create_string_interner();
        let (arena, offsets) = interner.export_arena();

        assert_eq!(arena, "");
        assert_eq!(offsets, alloc::vec![0]); // Should just contain the initial 0
    }

    #[test]
    fn test_lookup_handle_non_existent() {
        let interner = create_string_interner();
        // Explicitly test the Ok(None) path which maps through the Option
        let res = interner.lookup_handle("ghost");
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
    }
    #[test]
    fn test_default_impl() {
        // Covers: impl Default for Interner
        let interner: Interner<String, RandomState> = Interner::default();
        assert!(interner.is_empty());
    }

    #[test]
    fn test_explicit_iter() {
        // Covers: pub fn iter(&self)
        let mut interner = create_string_interner();
        interner.intern_ref("A").unwrap();

        // Explicitly call .iter() instead of relying on IntoIterator
        let mut iter = interner.iter();
        assert_eq!(iter.next(), Some(&"A".to_string()));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_error_debug_impl() {
        // Covers: #[derive(Debug)] for InternerError
        let err = InternerError::Overflow;
        let debug_output = alloc::format!("{err:?}");
        assert_eq!(debug_output, "Overflow");
    }

    #[test]
    fn test_lookup_handle_success() {
        // Covers: The 'Some' path of lookup_handle logic
        let mut interner = create_string_interner();
        let h = interner.intern_ref("A").unwrap();

        let found = interner.lookup_handle("A").unwrap();
        assert_eq!(found, Some(h));
    }

    #[test]
    fn test_remove_handle_shifts_indices() {
        let mut interner = create_string_interner();

        // 1. Insert [A, B, C]
        let h_a = interner.intern_ref("A").unwrap(); // 0
        let h_b = interner.intern_ref("B").unwrap(); // 1
        let h_c = interner.intern_ref("C").unwrap(); // 2

        assert_eq!(interner.len(), 3);

        // 2. Remove "B" (index 1) using its handle
        let removed = interner.remove_handle(h_b);

        assert_eq!(removed, Some("B".to_string()));
        assert_eq!(interner.len(), 2);

        // 3. Verify the state of the remaining handles

        // Handle 0 ("A") is unaffected because it was *before* the removal.
        assert_eq!(interner.resolve(h_a), Some(&"A".to_string()));

        // Handle 2 ("C") is now BROKEN. It points to index 2, but the vector
        // is only length 2 (indices 0 and 1).
        assert_eq!(interner.resolve(h_c), None);

        // "C" has actually shifted down to Handle 1.
        // (This simulates what happens if we reused the old 'B' handle)
        assert_eq!(interner.resolve(h_b), Some(&"C".to_string()));
    }

    #[test]
    fn test_remove_and_recover_handles() {
        let mut interner = create_string_interner();

        // 1. Setup handles: [0, 1, 2, 3]
        // Items: ["A", "B", "C", "D"]
        let mut handles = alloc::vec![
            interner.intern_ref("A").unwrap(), // 0
            interner.intern_ref("B").unwrap(), // 1
            interner.intern_ref("C").unwrap(), // 2
            interner.intern_ref("D").unwrap(), // 3
        ];

        // 2. Remove "B" (index 1).
        // usage: remove returns the handle of the item that was removed.
        let (removed_handle, val) = interner.remove("B").unwrap();

        assert_eq!(val, "B");
        assert_eq!(removed_handle, 1);

        // 3. The Recovery Loop
        // We iterate over our local handles and patch them.
        for h in &mut handles {
            // Use strict greater-than (>).
            // Handles < 1 stay the same.
            // Handle == 1 is the one we just removed.
            if *h > removed_handle {
                *h -= 1;
            }
        }

        // 4. Verification

        // "A" (was 0) should still be 0
        assert_eq!(interner.resolve(handles[0]), Some(&"A".to_string()));

        // "B" (was 1) was removed. In our vector, `handles[1]` is still `1`.
        // However, in the interner, index 1 has been filled by "C".
        // This is expected behavior for the "removed" handle.
        assert_eq!(interner.resolve(handles[1]), Some(&"C".to_string()));

        // "C" (was 2) should have been patched to 1.
        assert_eq!(handles[2], 1);
        assert_eq!(interner.resolve(handles[2]), Some(&"C".to_string()));

        // "D" (was 3) should have been patched to 2.
        assert_eq!(handles[3], 2);
        assert_eq!(interner.resolve(handles[3]), Some(&"D".to_string()));
    }

    #[test]
    fn test_remove_and_recover_handles_helper() {
        let mut interner = create_string_interner();

        let mut handles = alloc::vec![
            interner.intern_ref("A").unwrap(), // 0
            interner.intern_ref("B").unwrap(), // 1
            interner.intern_ref("C").unwrap(), // 2
            interner.intern_ref("D").unwrap(), // 3
        ];

        // 1. Remove "B" (index 1).
        let (removed_handle, val) = interner.remove("B").unwrap();
        assert_eq!(val, "B");

        // 2. REPAIR AUTOMATICALLY
        // We pass a mutable reference to the vector (which is IntoIterator)
        interner.repair_handles(removed_handle, &mut handles);

        // 3. Verify
        assert_eq!(interner.resolve(handles[0]), Some(&"A".to_string()));
        assert_eq!(interner.resolve(handles[1]), Some(&"C".to_string())); // Was 1, still 1, now points to C
        assert_eq!(interner.resolve(handles[2]), Some(&"C".to_string())); // Was 2, fixed to 1, points to C
        assert_eq!(interner.resolve(handles[3]), Some(&"D".to_string())); // Was 3, fixed to 2, points to D
    }

    #[test]
    fn test_repair_handles_in_structs() {
        struct User {
            name_handle: u32,
            _score: i32,
        }

        let mut interner = create_string_interner();
        let h_a = interner.intern_ref("A").unwrap(); // 0
        let h_b = interner.intern_ref("B").unwrap(); // 1
        let h_c = interner.intern_ref("C").unwrap(); // 2

        let mut users = alloc::vec![
            User {
                name_handle: h_a,
                _score: 10,
            },
            User {
                name_handle: h_b,
                _score: 20,
            },
            User {
                name_handle: h_c,
                _score: 30,
            },
        ];

        // Remove "A" (Handle 0). Everything > 0 should shift down.
        let (removed, _) = interner.remove("A").unwrap();

        // Complex usage: map to the field
        interner.repair_handles(removed, users.iter_mut().map(|u| &mut u.name_handle));

        // Validation
        // A was removed.
        // B (was 1) should become 0.
        // C (was 2) should become 1.

        assert_eq!(users[1].name_handle, 0);
        assert_eq!(
            interner.resolve(users[1].name_handle),
            Some(&"B".to_string())
        );

        assert_eq!(users[2].name_handle, 1);
        assert_eq!(
            interner.resolve(users[2].name_handle),
            Some(&"C".to_string())
        );
    }
}
