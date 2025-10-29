## Changelog

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
