extern crate alloc;

use alloc::{
    borrow::ToOwned as _,
    boxed::Box,
    ffi::CString,
    rc::Rc,
    string::{String, ToString as _},
    sync::Arc,
    vec::Vec,
};
use core::ffi::CStr;

/// Construct an owned type from a reference.
///
/// Similar to [`ToOwned`] or [`Clone`], but it can be implemented on any
/// combination of base type and `Borrowed` type.
pub trait FromRef<Borrowed: ?Sized> {
    /// Construct an owned type from a reference.
    fn from_ref(val: &Borrowed) -> Self;
}

// str
impl FromRef<str> for Box<str> {
    fn from_ref(val: &str) -> Self {
        Box::from(val)
    }
}
impl FromRef<str> for Rc<str> {
    fn from_ref(val: &str) -> Self {
        Rc::from(val)
    }
}
impl FromRef<str> for Arc<str> {
    fn from_ref(val: &str) -> Self {
        Arc::from(val)
    }
}
impl FromRef<str> for String {
    fn from_ref(val: &str) -> Self {
        val.to_string()
    }
}

// CStr
impl FromRef<CStr> for Box<CStr> {
    fn from_ref(val: &CStr) -> Self {
        Box::from(val)
    }
}
impl FromRef<CStr> for Rc<CStr> {
    fn from_ref(val: &CStr) -> Self {
        Rc::from(val)
    }
}
impl FromRef<CStr> for Arc<CStr> {
    fn from_ref(val: &CStr) -> Self {
        Arc::from(val)
    }
}
impl FromRef<CStr> for CString {
    fn from_ref(val: &CStr) -> Self {
        val.to_owned()
    }
}

// T
impl<T: Clone> FromRef<T> for T {
    fn from_ref(val: &T) -> Self {
        val.clone()
    }
}

// [T]
impl<T: Clone> FromRef<[T]> for Box<[T]> {
    fn from_ref(val: &[T]) -> Self {
        Box::from(val)
    }
}
impl<T: Clone> FromRef<[T]> for Rc<[T]> {
    fn from_ref(val: &[T]) -> Self {
        Rc::from(val)
    }
}
impl<T: Clone> FromRef<[T]> for Arc<[T]> {
    fn from_ref(val: &[T]) -> Self {
        Arc::from(val)
    }
}
impl<T: Clone> FromRef<[T]> for Vec<T> {
    fn from_ref(val: &[T]) -> Self {
        val.to_vec()
    }
}

// Gate the OS-specific ones
#[cfg(feature = "std")]
mod os_impls {
    extern crate std;

    use alloc::{boxed::Box, rc::Rc, sync::Arc};
    use std::{
        ffi::{OsStr, OsString},
        path::{Path, PathBuf},
    };

    use super::FromRef;

    // OsStr
    impl FromRef<OsStr> for Box<OsStr> {
        fn from_ref(val: &OsStr) -> Self {
            Box::from(val)
        }
    }
    impl FromRef<OsStr> for Rc<OsStr> {
        fn from_ref(val: &OsStr) -> Self {
            Rc::from(val)
        }
    }
    impl FromRef<OsStr> for Arc<OsStr> {
        fn from_ref(val: &OsStr) -> Self {
            Arc::from(val)
        }
    }
    impl FromRef<OsStr> for OsString {
        fn from_ref(val: &OsStr) -> Self {
            val.to_os_string()
        }
    }

    // Path
    impl FromRef<Path> for Box<Path> {
        fn from_ref(val: &Path) -> Self {
            Box::from(val)
        }
    }
    impl FromRef<Path> for Rc<Path> {
        fn from_ref(val: &Path) -> Self {
            Rc::from(val)
        }
    }
    impl FromRef<Path> for Arc<Path> {
        fn from_ref(val: &Path) -> Self {
            Arc::from(val)
        }
    }
    impl FromRef<Path> for PathBuf {
        fn from_ref(val: &Path) -> Self {
            val.to_path_buf()
        }
    }
}
