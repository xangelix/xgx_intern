## Changelog

### 0.5.1

**Features:**

- **Item Removal:** Added `remove` and `remove_handle` methods to allow deleting items from the interner.
  - **Note:** These operations are **O(n)** (linear time) as they require shifting the underlying vector to preserve order.
  - **Warning:** Removal **invalidates** existing handles that are greater than the removed index.
- **Handle Recovery:** Added the `repair_handles` helper method.
  - This utility iterates over a user-provided collection of handles (e.g., `&mut Vec<H>`) and automatically updates them to account for the index shift caused by a removal operation.

### 0.5.0

**Features:**

- **`no_std` Support:** The crate is now `no_std` compatible! It relies on the `alloc` crate for heap types (`String`, `Vec`, `Arc`, etc.).
  - By default, the `std` feature is enabled, preserving full functionality.
  - Users can disable default features to use the crate in embedded or kernel environments.
- **Feature Gating:**
  - `Path`, `PathBuf`, `OsStr`, and `OsString` implementations are now guarded behind the `std` feature.
  - `RandomState` (default hasher) requires the `std` feature. `no_std` users must supply their own hasher (e.g., `ahash`) or explicit build hasher.
- **More README examples**

**Dependency Updates:**

- `indexmap` and `thiserror` are now configured with `default-features = false` to support the `no_std` ecosystem.

### 0.4.0

**Features:**

- **Smart Pointer Support:** You can now intern `Arc<str>`, `Rc<str>`, and `Box<str>` directly from `&str` using `intern_ref`. This enables zero-allocation lookups for shared strings!
- **OS/FFI Type Support:** Added support for interning `Path` / `PathBuf`, `OsStr` / `OsString`, and `CStr` / `CString`.
- **`FromRef` Trait:** Added the `FromRef` trait, allowing users to define custom reference-to-owned conversions for their own types.

**Breaking Changes:**

- `intern_ref` now relies on a new trait, `FromRef`, instead of `std::borrow::ToOwned`. This allows for more flexible interning of types where the borrowed form and owned form differ in ways `ToOwned` cannot handle (e.g., `&str` -> `Arc<str>`).
  - If you were interning standard types (`String`, `Vec`, `PathBuf`, etc.) or simple `Clone`-able structs, no changes are required.
  - If you were interning custom types relying on a custom `ToOwned` implementation, you must now implement `FromRef` for your type.

#### Migration Example

If you previously had:

```rust
// Old way relying on ToOwned
impl ToOwned for MyRef {
    type Owned = MyType;

    fn to_owned(&self) -> MyType { ... }
}

```

Rewrite to:

```rust
// New way relying on FromRef
impl FromRef<MyRef> for MyType {
    fn from_ref(val: &MyRef) -> Self {
        val.to_owned() // or custom logic
    }
}

```

### 0.3.8

- Add arena export, `export_arena`, option

### 0.3.7

- Implement `Debug` for `Interner`
- Float helpers: `new`, `into_inner`, `as_inner`
- Add `intern_ref_or_insert_with`

### 0.3.6

- Update changelog for v0.3.5 and v0.3.6

### 0.3.5

- Add `lookup_handle` to fetch a handle without inserting.
- Add iteration ergonomics: `iter()` and `IntoIterator` for `&Interner` and by-value.
- Add utilities: `contains`, `capacity`, `reserve`, `shrink_to_fit`, `clear`.
- Implement `Default` when `BuildHasher: Default`.
- Re-export `HashableF32`/`HashableF64` at the crate root.
- Float wrappers: implement `Debug` and `Display`.
- `export()`: add `into_vec` doc alias.
- Tests: add `ahash` doc test and more tests for new APIs.
- Docs: clarify uniqueness wording and improve the tagline.

### 0.3.4

- Many small fixes and cleanups to README.md

### 0.3.3

- Add a couple wasm32 handle size warnings to README.md

### 0.3.2

- Enforce docs on all items
- Add docs to all items
- Add basic shields/badges to top of README.md
- Fill out README.md with examples and more details

### 0.3.1

- Add author Cargo.toml metadata
- Add keyword and category Cargo.toml metadata

### 0.3.0

- Support any index type that can be converted into a usize on the platform
- Support internment on types without the `Clone` trait, with specific intern_owned function
- Add LICENSE
- Change license to MIT
- Add many doc comments
- Add many test cases
- Support borrowed types (e.g. &str)
- Update README.md on latest features
- Add CHANGELOG.md
- Upgrade rustc-hash
