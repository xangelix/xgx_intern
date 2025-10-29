use std::{
    fmt,
    hash::{Hash, Hasher},
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

impl std::ops::Deref for HashableF64 {
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

impl std::ops::Deref for HashableF32 {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
