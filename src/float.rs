use core::{
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
};

/// A wrapper around f64 that implements Eq and Hash based on bit patterns.
#[derive(Clone, Copy, Debug, PartialOrd)]
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

impl fmt::Display for HashableF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
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

impl HashableF64 {
    /// Creates a new `HashableF64` from an f64 value.
    #[must_use]
    #[inline]
    pub const fn new(value: f64) -> Self {
        Self(value)
    }
    /// Consumes the `HashableF64` and returns the inner f64 value.
    #[must_use]
    #[inline]
    pub const fn into_inner(self) -> f64 {
        self.0
    }
    /// Returns a reference to the inner f64 value.
    #[must_use]
    #[inline]
    pub const fn as_inner(&self) -> &f64 {
        &self.0
    }
}

impl Deref for HashableF64 {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A wrapper around f32 that implements Eq and Hash based on bit patterns.
#[derive(Clone, Copy, Debug, PartialOrd)]
pub struct HashableF32(pub f32);

impl PartialEq for HashableF32 {
    fn eq(&self, other: &Self) -> bool {
        // Two floats are equal if and only if their bit patterns are identical.
        // This means 0.0 and -0.0 are treated as different, and NaN == NaN.
        self.0.to_bits() == other.0.to_bits()
    }
}

// Since we've defined a total equality relation, we can implement Eq.
impl Eq for HashableF32 {}

impl Hash for HashableF32 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the underlying bits of the float.
        self.0.to_bits().hash(state);
    }
}

impl fmt::Display for HashableF32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<HashableF32> for f32 {
    fn from(value: HashableF32) -> Self {
        value.0
    }
}

impl From<f32> for HashableF32 {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl HashableF32 {
    /// Creates a new `HashableF32` from an f32 value.
    #[must_use]
    #[inline]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }
    /// Consumes the `HashableF32` and returns the inner f32 value.
    #[must_use]
    #[inline]
    pub const fn into_inner(self) -> f32 {
        self.0
    }
    /// Returns a reference to the inner f32 value.
    #[must_use]
    #[inline]
    pub const fn as_inner(&self) -> &f32 {
        &self.0
    }
}

impl Deref for HashableF32 {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use core::hash::{Hash as _, Hasher as _};

    use super::{HashableF32, HashableF64};

    #[test]
    fn hashable_f32_nan_equality_and_hash() {
        let a = HashableF32(f32::NAN);
        let b = HashableF32(f32::from_bits(f32::NAN.to_bits()));
        assert_eq!(a, b);

        let mut ha = ahash::AHasher::default();
        let mut hb = ahash::AHasher::default();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }

    #[test]
    fn hashable_f64_nan_equality_and_hash() {
        let a = HashableF64(f64::NAN);
        let b = HashableF64(f64::from_bits(f64::NAN.to_bits()));
        assert_eq!(a, b);

        let mut ha = ahash::AHasher::default();
        let mut hb = ahash::AHasher::default();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }

    #[test]
    fn hashable_f64_signed_zero_unequal() {
        let pz = HashableF64(0.0);
        let nz = HashableF64(-0.0);
        assert_ne!(pz, nz);
    }

    #[allow(clippy::float_cmp)]
    #[test]
    fn test_f32_wrapper_comprehensive() {
        let raw = 1.23f32;
        let wrapped = HashableF32::new(raw);

        // Test Deref
        assert_eq!(*wrapped, raw);
        // Test as_inner
        assert_eq!(wrapped.as_inner(), &raw);
        // Test into_inner
        assert_eq!(wrapped.into_inner(), raw);

        // Re-create for trait tests
        let wrapped = HashableF32::new(raw);

        // Test From<f32>
        let from_raw: HashableF32 = raw.into();
        assert_eq!(from_raw, wrapped);

        // Test From<Wrapper> for f32
        let back_to_raw: f32 = wrapped.into();
        assert_eq!(back_to_raw, raw);

        // Test Display
        assert_eq!(format!("{wrapped}"), "1.23");

        // Test PartialOrd
        assert!(HashableF32::new(1.0) < HashableF32::new(2.0));
    }

    #[allow(clippy::float_cmp)]
    #[test]
    fn test_f64_wrapper_comprehensive() {
        let raw = 4.56f64;
        let wrapped = HashableF64::new(raw);

        // Test Deref
        assert_eq!(*wrapped, raw);
        // Test as_inner
        assert_eq!(wrapped.as_inner(), &raw);
        // Test into_inner
        assert_eq!(wrapped.into_inner(), raw);

        // Re-create for trait tests
        let wrapped = HashableF64::new(raw);

        // Test From<f64>
        let from_raw: HashableF64 = raw.into();
        assert_eq!(from_raw, wrapped);

        // Test From<Wrapper> for f64
        let back_to_raw: f64 = wrapped.into();
        assert_eq!(back_to_raw, raw);

        // Test Display
        assert_eq!(format!("{wrapped}"), "4.56");

        // Test PartialOrd
        assert!(HashableF64::new(1.0) < HashableF64::new(2.0));
    }

    #[test]
    fn test_special_values() {
        // NaN equality check
        let nan32 = HashableF32::new(f32::NAN);
        assert_eq!(nan32, nan32); // Eq holds

        let nan64 = HashableF64::new(f64::NAN);
        assert_eq!(nan64, nan64); // Eq holds

        // Signed zero check
        let pz = HashableF32::new(0.0);
        let nz = HashableF32::new(-0.0);
        assert_ne!(pz, nz); // Bitwise difference
    }

    // Covers: #[derive(Debug)] for HashableF32 and HashableF64
    #[test]
    fn test_debug_impls() {
        let f32_val = HashableF32::new(1.0);
        let f64_val = HashableF64::new(1.0);

        assert_eq!(format!("{f32_val:?}"), "HashableF32(1.0)");
        assert_eq!(format!("{f64_val:?}"), "HashableF64(1.0)");
    }

    // Covers: Derived PartialOrd branches (>, >=) that weren't hit by just '<'
    #[test]
    fn test_ordering_completeness() {
        let small = HashableF32::new(1.0);
        let big = HashableF32::new(2.0);

        assert!(big > small);
        assert!(big >= small);
        assert!(small <= big);

        let small64 = HashableF64::new(1.0);
        let big64 = HashableF64::new(2.0);

        assert!(big64 > small64);
        assert!(big64 >= small64);
    }

    // Covers: #[derive(Clone)] explicitly
    #[allow(clippy::clone_on_copy)]
    #[test]
    fn test_explicit_clone() {
        let a = HashableF32::new(1.0);
        let b = a.clone();
        assert_eq!(a, b);

        let c = HashableF64::new(1.0);
        let d = c.clone();
        assert_eq!(c, d);
    }
}
