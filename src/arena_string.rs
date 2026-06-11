extern crate alloc;

use alloc::sync::Arc;
use core::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
};

#[cfg(feature = "compact_str")]
use compact_str::CompactString;

use crate::FromRef;

/// A memory-efficient string representation designed for zero-allocation
/// snapshot deserialization and low-overhead value interning.
///
/// It can represent either a slice inside a shared immutable arena
/// (`Arc<str>`) or a fallback owned dynamic allocation (`Box<str>` or
/// `CompactString` if the `compact_str` feature is enabled).
#[derive(Clone)]
pub enum ArenaString {
    /// A sliced string referencing a shared memory arena.
    Shared {
        /// The shared string arena backing buffer.
        arena: Arc<str>,
        /// The starting byte offset within the arena.
        offset: u32,
        /// The byte length of the string slice.
        len: u32,
    },
    /// A dynamically allocated owned string fallback.
    #[cfg(feature = "compact_str")]
    Owned(CompactString),
    /// A dynamically allocated owned string fallback.
    #[cfg(not(feature = "compact_str"))]
    Owned(alloc::boxed::Box<str>),
}

impl ArenaString {
    /// Returns a reference to the underlying string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Shared { arena, offset, len } => {
                let start = *offset as usize;
                let end = start + *len as usize;
                arena.get(start..end).unwrap_or("")
            }
            #[cfg(feature = "compact_str")]
            Self::Owned(s) => s.as_str(),
            #[cfg(not(feature = "compact_str"))]
            Self::Owned(s) => s,
        }
    }
}

impl PartialEq for ArenaString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for ArenaString {}

impl PartialOrd for ArenaString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ArenaString {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for ArenaString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl Borrow<str> for ArenaString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for ArenaString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for ArenaString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for ArenaString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl FromRef<str> for ArenaString {
    #[inline]
    fn from_ref(val: &str) -> Self {
        #[cfg(feature = "compact_str")]
        {
            Self::Owned(CompactString::new(val))
        }
        #[cfg(not(feature = "compact_str"))]
        {
            Self::Owned(Box::from(val))
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use ahash::RandomState;

    use super::ArenaString;
    use crate::Interner;

    #[test]
    fn test_arena_string_shared() {
        let data: Arc<str> = Arc::from("helloworld");
        let s1 = ArenaString::Shared {
            arena: data.clone(),
            offset: 0,
            len: 5,
        };
        let s2 = ArenaString::Shared {
            arena: data,
            offset: 5,
            len: 5,
        };

        assert_eq!(s1.as_str(), "hello");
        assert_eq!(s2.as_str(), "world");
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_arena_string_interning() {
        let mut interner = Interner::<ArenaString, RandomState>::new(RandomState::new());

        // Interning via references (creates Owned variants)
        let h1 = interner.intern_ref("hello").unwrap();
        let h2 = interner.intern_ref("world").unwrap();
        let h3 = interner.intern_ref("hello").unwrap();

        assert_eq!(h1, h3);
        assert_ne!(h1, h2);

        assert_eq!(interner.resolve(h1).unwrap().as_str(), "hello");

        // Interning via pre-constructed Shared variants
        let shared_arena: Arc<str> = Arc::from("helloworld");
        let shared_item = ArenaString::Shared {
            arena: shared_arena,
            offset: 0,
            len: 5,
        };

        // This matches the existing "hello" (since Eq and Hash match)
        let h4 = interner.intern_owned(shared_item).unwrap();
        assert_eq!(h1, h4);
    }
}
