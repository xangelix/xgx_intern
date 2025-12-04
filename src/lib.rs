#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Provides wrappers for interning floating-point types.
///
/// Standard `f32` and `f64` types do not implement `Eq` or `Hash` due to `NaN` semantics,
/// making them unusable with `Interner` directly. This module offers custom
/// types that provide a canonical representation for hashing and equality, allowing
/// floats to be reliably interned.
pub mod float;

use std::{
    borrow::{Borrow, Cow},
    fmt,
    hash::{BuildHasher, Hash},
    marker::PhantomData,
};

use indexmap::IndexSet;

pub use crate::float::{HashableF32, HashableF64};

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
    /// This method requires `T: Clone` and is ideal for cases where you have a
    /// reference to a value and want to avoid cloning it if it's already been
    /// interned.
    ///
    /// # Errors
    ///
    /// Returns `InternerError::Overflow` if a new item is inserted and the
    /// interner's handle capacity is exhausted.
    pub fn intern_ref<Q>(&mut self, item: &Q) -> Result<H, InternerError>
    where
        T: Borrow<Q> + Clone,
        Q: ToOwned<Owned = T> + Hash + Eq + ?Sized,
    {
        if let Some(idx) = self.items.get_index_of(item) {
            return Self::idx_to_handle(idx);
        }
        let h = Self::idx_to_handle(self.items.len())?;
        self.items.insert(item.to_owned());
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
    use super::*;
    use rustc_hash::FxHasher;
    use std::collections::hash_map::RandomState;
    use std::hash::BuildHasherDefault;

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

        let expected = vec!["first".to_string(), "second".to_string()];
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

        assert_eq!(collected, vec!["a", "b"]);
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
    fn hashable_f64_nan_equality_and_hash() {
        use std::{collections::hash_map::DefaultHasher, hash::Hasher as _};

        use crate::HashableF64;
        let a = HashableF64(f64::NAN);
        let b = HashableF64(f64::from_bits(f64::NAN.to_bits()));
        assert_eq!(a, b);

        let mut ha = DefaultHasher::new();
        let mut hb = DefaultHasher::new();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }

    #[test]
    fn hashable_f64_signed_zero_unequal() {
        use crate::HashableF64;
        let pz = HashableF64(0.0);
        let nz = HashableF64(-0.0);
        assert_ne!(pz, nz);
    }
}
