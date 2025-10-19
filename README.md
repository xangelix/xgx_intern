## xgx_intern

[![Crates.io](https://img.shields.io/crates/v/xgx_intern)](https://crates.io/crates/xgx_intern)
[![Docs.rs](https://docs.rs/xgx_intern/badge.svg)](https://docs.rs/xgx_intern)
[![License](https://img.shields.io/crates/l/xgx_intern)](https://spdx.org/licenses/MIT)

A high-performance, Hash-based value interner with custom handle types.

Supports any type that implements the Hash trait for internment, and allows custom handle sizes! Perfect for native64<-->wasm32 compatibility.

## Overview

Value interning is a technique for deduplicating equal values to save memory and improve performance. An interner stores each unique value only once and provides a lightweight, copyable "handle" (or "symbol") to reference it.

This approach offers two main benefits:

1.  **Memory Efficiency**: If you have many duplicate values (e.g., strings in a compiler's AST, repeated keys in a dataset), interning ensures only one copy of each unique value is stored in memory.
2.  **Performance Boost**: Comparing two interned values becomes an extremely fast integer comparison (handle vs. handle) instead of a potentially expensive deep comparison (e.g., string vs. string).

`xgx_intern` provides a flexible and ergonomic implementation of this pattern, suitable for a wide range of applications.

## Features

- **Fully Generic**: Works with any type that implements `Eq + Hash`.
- **Customizable Hasher**: Pluggable hashing algorithm via the `BuildHasher` trait. Use `ahash` or `fxhash` for a significant speed boost in performance-critical code.
- **Customizable Handle**: Choose the integer size for your handles (`u16`, `u32`, `u64`, etc.) to perfectly balance memory usage with the expected number of unique items.
- **Ergonomic API**: Offers `intern_owned`, `intern_ref`, and `intern_cow` to handle different ownership scenarios efficiently and avoid unnecessary clones.
- **Float Support**: Includes `HashableF32` and `HashableF64` wrappers to enable reliable interning of floating-point numbers, which don't normally implement `Eq` or `Hash`.
- **Order Preserving**: Built on `indexmap`, the interner preserves the insertion order of unique values.
- **Export**: Done interning values? Export the whole thing to a `Vec<T>` for further simplicity and memory efficiency.

> **⚠️ WebAssembly Note:** When compiling for a `wasm32` target, it's **critical** that you use a handle size of `u32` or smaller (`u16`, `u8`). The `wasm32` architecture has a 32-bit pointer size (`usize`), so it cannot create handles from larger types like `u64`, which would cause an error.

## Installation

Add `xgx_intern` to your `Cargo.toml`:

```toml
[dependencies]
xgx_intern = "0.3" # make sure to use latest version
```

For improved performance, you can use a faster hasher like `ahash`:

```toml
[dependencies]
ahash = "0.8" # make sure to use latest version
```

but make sure you understand the security and safety tradeoffs in your use case.

## Usage

### Example: Interning Strings

This is the most common use case. Here, we intern several strings and observe how duplicates are handled.

```rust
use std::collections::hash_map::RandomState;

use xgx_intern::Interner;

// Create an interner for strings with the default hasher and u32 handles.
let mut interner = Interner::<String, _>::new(RandomState::new());

// Intern some strings. `intern_ref` clones the data only if it's not already present.
let handle1 = interner.intern_ref("hello").unwrap();
let handle2 = interner.intern_ref("world").unwrap();
let handle3 = interner.intern_ref("hello").unwrap(); // This is a duplicate

// Handles for identical values are guaranteed to be the same.
assert_eq!(handle1, handle3);
assert_ne!(handle1, handle2);

// Even though we interned three values, the interner only stores two unique strings.
assert_eq!(interner.len(), 2);

// You can resolve a handle back to the original value for inspection.
assert_eq!(interner.resolve(handle1), Some(&"hello".to_string()));

println!("Handle {:?} resolved to '{}'", handle1, interner.resolve(handle1).unwrap());
// Output: Handle 0 resolved to 'hello'
```

### Example: Interning a Custom Struct

Any type that implements `Eq`, `PartialEq`, `Hash`, and `Clone` (for `intern_ref`) can be interned.

```rust
use std::collections::hash_map::RandomState;
use xgx_intern::Interner;

// 1. Define a custom type that can be interned.
//    Deriving these traits is usually sufficient.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct User {
    id: u32,
    username: String,
}

// 2. Create an interner for your custom type.
let mut interner = Interner::<User, _>::new(RandomState::new());

// 3. Intern instances of your struct.
let user1 = User { id: 101, username: "alice".to_string() };
let user2 = User { id: 102, username: "bob".to_string() };
let user3 = User { id: 101, username: "alice".to_string() }; // A duplicate of user1

let h1 = interner.intern_ref(&user1).unwrap();
let h2 = interner.intern_ref(&user2).unwrap();
let h3 = interner.intern_ref(&user3).unwrap();

// Assert that the duplicate user gets the same handle.
assert_eq!(h1, h3);
assert_ne!(h1, h2);
assert_eq!(interner.len(), 2);

// Resolve the handle to get a reference to the stored user.
let resolved_user = interner.resolve(h1).unwrap();
println!("Found user: {:?}", resolved_user);
// Output: Found user: User { id: 101, username: "alice" }
```

## Customization

### Using a Faster Hasher

The default `RandomState` hasher is secure but can be slow. For contexts where DOS resistance is not a concern, a faster non-cryptographic hasher like `ahash` is an excellent choice.

First, add `ahash` to your `Cargo.toml`:

```toml
[dependencies]
ahash = "0.8"
```

Then, use its `RandomState` (which implements `BuildHasher`) to create the interner:

```rust
use ahash::RandomState;
use xgx_intern::Interner;

// Create an interner that uses the fast `ahash` algorithm.
let mut interner = Interner::<String, RandomState>::new(RandomState::new());

let handle = interner.intern_owned("even faster hashing!".to_string()).unwrap();

println!("Interned with ahash and got handle: {:?}", handle);
```

You can see more rust hash benchmarks here: [Rust Hash Benchmarks](https://github.com/ogxd/gxhash?tab=readme-ov-file#benchmarks).

### Choosing a Handle Size

The default handle type `H` is `u32`, which allows for up to \~4.2 billion unique items. If you know you'll have fewer unique items, you can use a smaller handle type like `u16` to save memory.

```rust
use std::collections::hash_map::RandomState;
use xgx_intern::Interner;

// This interner uses u16 handles, limiting it to 65,536 unique items.
// This is perfect for smaller-scale problems and saves memory for each handle.
let mut interner = Interner::<String, RandomState, u16>::new(RandomState::new());

// The returned handles will now be of type `u16`.
let handle: u16 = interner.intern_ref("small").unwrap();

assert_eq!(handle, 0);
```

Conversely, if you need more than `u32::MAX` items, you can use `u64`.

> **⚠️ WebAssembly Note:** When compiling for a `wasm32` target, it's **critical** that you use a handle size of `u32` or smaller (`u16`, `u8`). The `wasm32` architecture has a 32-bit pointer size (`usize`), so it cannot create handles from larger types like `u64`, which would cause an error.

## License

This project is licensed under the ([LICENSE-MIT](https://spdx.org/licenses/MIT)).
