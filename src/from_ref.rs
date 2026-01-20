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
        Self::from(val)
    }
}
impl FromRef<str> for Rc<str> {
    fn from_ref(val: &str) -> Self {
        Self::from(val)
    }
}
impl FromRef<str> for Arc<str> {
    fn from_ref(val: &str) -> Self {
        Self::from(val)
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
        Self::from(val)
    }
}
impl FromRef<CStr> for Rc<CStr> {
    fn from_ref(val: &CStr) -> Self {
        Self::from(val)
    }
}
impl FromRef<CStr> for Arc<CStr> {
    fn from_ref(val: &CStr) -> Self {
        Self::from(val)
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
        Self::from(val)
    }
}
impl<T: Clone> FromRef<[T]> for Rc<[T]> {
    fn from_ref(val: &[T]) -> Self {
        Self::from(val)
    }
}
impl<T: Clone> FromRef<[T]> for Arc<[T]> {
    fn from_ref(val: &[T]) -> Self {
        Self::from(val)
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
            Self::from(val)
        }
    }
    impl FromRef<OsStr> for Rc<OsStr> {
        fn from_ref(val: &OsStr) -> Self {
            Self::from(val)
        }
    }
    impl FromRef<OsStr> for Arc<OsStr> {
        fn from_ref(val: &OsStr) -> Self {
            Self::from(val)
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
            Self::from(val)
        }
    }
    impl FromRef<Path> for Rc<Path> {
        fn from_ref(val: &Path) -> Self {
            Self::from(val)
        }
    }
    impl FromRef<Path> for Arc<Path> {
        fn from_ref(val: &Path) -> Self {
            Self::from(val)
        }
    }
    impl FromRef<Path> for PathBuf {
        fn from_ref(val: &Path) -> Self {
            val.to_path_buf()
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, ffi::CString, rc::Rc, string::String, sync::Arc, vec::Vec};
    use core::ffi::CStr;

    use ahash::RandomState;

    use crate::{FromRef, Interner};

    #[cfg(feature = "std")]
    #[test]
    fn test_from_ref_system_types() {
        extern crate std;
        use std::{
            ffi::{CString, OsStr, OsString},
            path::{Path, PathBuf},
        };

        let mut interner = Interner::<PathBuf, RandomState>::new(RandomState::new());
        let p = Path::new("/tmp/test");
        let h = interner.intern_ref(p).unwrap();
        assert_eq!(interner.resolve(h).unwrap(), p);

        let mut os_interner = Interner::<OsString, RandomState>::new(RandomState::new());
        let o = OsStr::new("test_os_str");
        let h_os = os_interner.intern_ref(o).unwrap();
        assert_eq!(os_interner.resolve(h_os).unwrap(), o);

        // Test CStr
        let mut c_interner = Interner::<CString, RandomState>::new(RandomState::new());
        let c = c"hello";
        let h_c = c_interner.intern_ref(c).unwrap();
        assert_eq!(c_interner.resolve(h_c).unwrap().as_c_str(), c);

        // Test Box<Path> specifically (different FromRef impl than PathBuf)
        let mut box_path_interner = Interner::<Box<Path>, RandomState>::new(RandomState::new());
        let h_bp = box_path_interner.intern_ref(p).unwrap();
        assert_eq!(&**box_path_interner.resolve(h_bp).unwrap(), p);
    }

    #[test]
    fn test_from_ref_identity() {
        // Test the `impl<T: Clone> FromRef<T> for T` block
        // We use u32, which is Copy/Clone
        let mut interner = Interner::<u32, RandomState>::new(RandomState::new());
        let val = 42u32;
        // intern_ref takes &Q. Here Q is u32. T is u32.
        // It should clone the integer.
        let h = interner.intern_ref(&val).unwrap();
        assert_eq!(*interner.resolve(h).unwrap(), 42);
    }

    #[test]
    fn test_from_ref_slices_generic() {
        // Test [T] -> Box<[T]>
        let mut interner = Interner::<Box<[u32]>, RandomState>::new(RandomState::new());
        let slice: &[u32] = &[1, 2, 3];
        let h = interner.intern_ref(slice).unwrap();
        assert_eq!(&**interner.resolve(h).unwrap(), slice);

        // Test [T] -> Rc<[T]>
        let mut rc_interner = Interner::<Rc<[u32]>, RandomState>::new(RandomState::new());
        let h_rc = rc_interner.intern_ref(slice).unwrap();
        assert_eq!(&**rc_interner.resolve(h_rc).unwrap(), slice);
    }

    // Helper to verify FromRef works for a specific type combo
    fn assert_from_ref<
        B: ?Sized + PartialEq + core::fmt::Debug,
        O: FromRef<B> + core::borrow::Borrow<B> + core::fmt::Debug + PartialEq,
    >(
        borrowed: &B,
        expected: &O,
    ) {
        let converted = O::from_ref(borrowed);
        assert_eq!(&converted, expected);
        assert_eq!(converted.borrow(), borrowed);
    }

    #[test]
    fn test_str_permutations() {
        let input = "hello";

        // Test String
        assert_from_ref::<str, String>(input, &String::from("hello"));

        // Test Box<str>
        let b: Box<str> = Box::from("hello");
        assert_from_ref::<str, Box<str>>(input, &b);

        // Test Rc<str>
        let r: Rc<str> = Rc::from("hello");
        assert_from_ref::<str, Rc<str>>(input, &r);

        // Test Arc<str>
        let a: Arc<str> = Arc::from("hello");
        assert_from_ref::<str, Arc<str>>(input, &a);
    }

    #[test]
    fn test_cstr_permutations() {
        let input = c"hello";

        // Test CString
        assert_from_ref::<CStr, CString>(input, &CString::new("hello").unwrap());

        // Test Box<CStr>
        let b: Box<CStr> = Box::from(input);
        assert_from_ref::<CStr, Box<CStr>>(input, &b);

        // Test Rc<CStr>
        let r: Rc<CStr> = Rc::from(input);
        assert_from_ref::<CStr, Rc<CStr>>(input, &r);

        // Test Arc<CStr>
        let a: Arc<CStr> = Arc::from(input);
        assert_from_ref::<CStr, Arc<CStr>>(input, &a);
    }

    #[test]
    fn test_slice_permutations() {
        let input: &[u8] = &[1, 2, 3];

        // Test Vec<T>
        assert_from_ref::<[u8], Vec<u8>>(input, &Vec::from(input));

        // Test Box<[T]>
        let b: Box<[u8]> = Box::from(input);
        assert_from_ref::<[u8], Box<[u8]>>(input, &b);

        // Test Rc<[T]>
        let r: Rc<[u8]> = Rc::from(input);
        assert_from_ref::<[u8], Rc<[u8]>>(input, &r);

        // Test Arc<[T]>
        let a: Arc<[u8]> = Arc::from(input);
        assert_from_ref::<[u8], Arc<[u8]>>(input, &a);
    }

    #[test]
    fn test_identity_t() {
        // Test T -> T (Clone)
        let input = 42;
        assert_from_ref::<i32, i32>(&input, &42);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_os_str_permutations() {
        extern crate std;
        use std::ffi::{OsStr, OsString};

        let input = OsStr::new("hello");

        // Test OsString
        assert_from_ref::<OsStr, OsString>(input, &OsString::from("hello"));

        // Test Box<OsStr>
        let b: Box<OsStr> = Box::from(input);
        assert_from_ref::<OsStr, Box<OsStr>>(input, &b);

        // Test Rc<OsStr>
        let r: Rc<OsStr> = Rc::from(input);
        assert_from_ref::<OsStr, Rc<OsStr>>(input, &r);

        // Test Arc<OsStr>
        let a: Arc<OsStr> = Arc::from(input);
        assert_from_ref::<OsStr, Arc<OsStr>>(input, &a);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_path_permutations() {
        extern crate std;
        use std::path::{Path, PathBuf};

        let input = Path::new("/tmp/hello");

        // Test PathBuf
        assert_from_ref::<Path, PathBuf>(input, &PathBuf::from("/tmp/hello"));

        // Test Box<Path>
        let b: Box<Path> = Box::from(input);
        assert_from_ref::<Path, Box<Path>>(input, &b);

        // Test Rc<Path>
        let r: Rc<Path> = Rc::from(input);
        assert_from_ref::<Path, Rc<Path>>(input, &r);

        // Test Arc<Path>
        let a: Arc<Path> = Arc::from(input);
        assert_from_ref::<Path, Arc<Path>>(input, &a);
    }
}
