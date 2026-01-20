# xgx_intern

[![Crates.io](https://img.shields.io/crates/v/xgx_intern)](https://crates.io/crates/xgx_intern)
[![Docs.rs](https://docs.rs/xgx_intern/badge.svg)](https://docs.rs/xgx_intern)
[![License](https://img.shields.io/crates/l/xgx_intern)](https://spdx.org/licenses/MIT)
[![no_std](https://img.shields.io/badge/no_std-8A2BE2)](https://docs.rust-embedded.org/book/intro/no-std.html)
![Coverage](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Fxangelix%2Fxgx_intern%2Fmain%2Fcoverage.json)


A high-performance, Hash-based value interner with custom handle types.

Supports any type that implements the Hash trait for internment, and allows custom handle sizes! Perfect for native64<-->wasm32 compatibility.

## 📖 Overview

Value interning is a technique for deduplicating equal values to save memory and improve performance. An interner stores each unique value only once and provides a lightweight, copyable "handle" (or "symbol") to reference it.

This approach offers two main benefits:

1.  **Memory Efficiency**: If you have many duplicate values (e.g., strings in a compiler's AST, repeated keys in a dataset), interning ensures only one copy of each unique value is stored in memory.
2.  **Performance Boost**: Comparing two interned values becomes an extremely fast integer comparison (handle vs. handle) instead of a potentially expensive deep comparison (e.g., string vs. string).

`xgx_intern` provides a flexible and ergonomic implementation of this pattern, suitable for a wide range of applications.

## ✨ Features

- 🧬 **Fully Generic**: Works with any type that implements `Eq + Hash`.
- ⚡ **Customizable Hasher**: Pluggable hashing algorithm via the `BuildHasher` trait. Use `ahash` or `fxhash` for a significant speed boost in performance-critical code.
- 🏷️ **Customizable Handle**: Choose the integer size for your handles (`u16`, `u32`, `u64`, etc.) to perfectly balance memory usage with the expected number of unique items.
- 🤲 **Ergonomic API**: Offers `intern_owned`, `intern_ref`, and `intern_cow` to handle different ownership scenarios efficiently and avoid unnecessary clones.
- 🧠 **Smart Pointer & System Types**: Efficiently interns `Arc<str>`, `Rc<str>`, `Box<str>`, `PathBuf`, `OsString`, and `CString` directly from borrowed references (`&str`, `&Path`, etc.), enabling zero-allocation lookups for shared strings and system types.
- 🔢 **Float Support**: Includes `HashableF32` and `HashableF64` wrappers to enable reliable interning of floating-point numbers, which don't normally implement `Eq` or `Hash`.
- 📋 **Order Preserving**: Built on `indexmap`, the interner preserves the insertion order of unique values.
- 📤 **Export**: Done interning values? Export the whole thing to a `Vec<T>` for further simplicity and memory efficiency.
- 🔌 **`no_std` Compatible**: Fully supports `no_std` environments via the `alloc` crate. Perfect for embedded systems, kernels, and WASM.

> **⚠️ WebAssembly Note:** When compiling for a `wasm32` target, it's **critical** that you use a handle size of `u32` or smaller (`u16`, `u8`). The `wasm32` architecture has a 32-bit pointer size (`usize`), so it cannot create handles from larger types like `u64`, which would cause an error.

## 📦 Installation

To add `xgx_intern` to your project, run:

```bash
cargo add xgx_intern
```

`xgx_intern` has just one feature, `std`, which enables support for native OS types.

### `no_std` Support

To use this crate in a `no_std` environment, disable the default features (disables the `std` feature).

```bash
cargo add xgx_intern --no-default-features
```

**Note:** In `no_std` mode, the default `RandomState` hasher is unavailable. You will likely want to add a `no_std` compatible hasher like `ahash`:
```bash
cargo add ahash --no-default-features
```

## 🚀 Usage

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

### Example: Zero-Allocation Shared Strings (`Arc<str>`)

This is a powerful pattern for high-performance applications like web servers or game engines.

Suppose you have a read-heavy system where many threads need to access shared tags or keys (e.g., "content-type", "player_name"). You want to store them as `Arc<str>` for cheap sharing, but you only have `&str` references from incoming network packets.

With standard `ToOwned`, looking up an `Arc<str>` would usually require allocating a temporary `String` first. **With `xgx_intern`, the lookup is allocation-free.**

```rust
use std::{collections::hash_map::RandomState, sync::Arc};

use xgx_intern::Interner;

// 1. Configure the interner to store `Arc<str>`.
//    This creates a deduplicated, thread-safe string cache.
let mut interner = Interner::<Arc<str>, RandomState>::new(RandomState::new());

// 2. Imagine we are parsing a file or network request.
//    We only have a borrowed slice, not an owned object.
let raw_input: &str = "application/json";

// 3. Intern the reference.
//    - If the value exists: We get the handle immediately. ZERO allocations.
//    - If the value is new: We allocate exactly ONE `Arc<str>`.
//
//    Without the `FromRef` trait, this would often require allocating a
//    temporary `String` just to perform the lookup.
let handle = interner.intern_ref(raw_input).unwrap();

// 4. Resolve the handle.
//    We get back a reference to the `Arc<str>` that lives in the interner.
let tag: &Arc<str> = interner.resolve(handle).unwrap();

assert_eq!(&**tag, "application/json");

// 5. Cheaply clone the Arc if you need to pass it to another thread.
let shared_tag = tag.clone();

```

## 💡 `FromRef` Trait Patterns

The `FromRef` trait is a superpower of this crate. Unlike the standard `ToOwned`, which rigidly maps a reference to its standard owned form (e.g., `&str` -> `String`), `FromRef` allows you to define **lazy transformations**.

This allows the interner to act as a **Deduplicating Cache** or a **Memoization Engine**, performing expensive work only when absolutely necessary.

### 1. Basic: Interning Custom Types

If you have a custom type that has a borrowed form (like `PathBuf` vs `Path` or your own wrapper types), you can enable `intern_ref` support by implementing the `FromRef` trait.

```rust
use std::{borrow::Borrow, collections::hash_map::RandomState, hash::Hash};

use xgx_intern::{Interner, FromRef};

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct PlayerId(u32);

// 1. Allow creating a PlayerId from a u32 ref (Required for insertion)
impl FromRef<u32> for PlayerId {
    fn from_ref(val: &u32) -> Self {
        PlayerId(*val)
    }
}

// 2. Allow viewing a PlayerId as a u32 (Required for lookup)
impl Borrow<u32> for PlayerId {
    fn borrow(&self) -> &u32 {
        &self.0
    }
}

fn main() {
    // Now you can intern using simple integers!
    let mut interner = Interner::<PlayerId, _>::new(RandomState::new());

    // The interner sees &100500, checks if any existing PlayerId borrows to that value.
    // If not, it uses from_ref to create a new PlayerId(100500).
    let handle = interner.intern_ref(&100500).unwrap();
}
```

### 2. Advanced: The Deduplicating Parser (Zero-Overhead Parsing)

**Scenario:** You are processing a stream of raw network packets or log lines. Many messages are identical.

**The Problem:** With a standard `HashMap`, you have to parse the bytes into a struct *before* you can check if you've seen it, wasting CPU on duplicates.

**The Solution:** `xgx_intern` looks up the raw bytes first. It triggers the parsing logic (via `FromRef`) only on a cache miss.

```rust
use std::{borrow::Borrow, hash::{Hash, Hasher}};

use serde::Deserialize;
use xgx_intern::{Interner, FromRef};

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
struct MarketData {
    // We keep the raw bytes to allow borrowing as &[u8] for lookups
    #[serde(skip)]
    raw: Vec<u8>, 
    ticker: String,
    price: u64,
}

// 1. Allow looking up MarketData using raw bytes
impl Borrow<[u8]> for MarketData {
    fn borrow(&self) -> &[u8] {
        &self.raw
    }
}

// 2. Hash based on the raw bytes (identity)
impl Hash for MarketData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

// 3. Define how to PARSE bytes into MarketData.
//    This runs ONLY if the packet is new.
impl FromRef<[u8]> for MarketData {
    fn from_ref(bytes: &[u8]) -> Self {
        // Expensive parsing step:
        let mut data: MarketData = serde_json::from_slice(bytes).unwrap();
        data.raw = bytes.to_vec();
        data
    }
}

fn main() {
    let mut cache = Interner::<MarketData, _>::new(std::collections::hash_map::RandomState::new());
    
    let packet = br#"{"ticker": "BTC", "price": 100000}"#;

    // First time: Cache miss. Calls from_ref. Allocates and parses JSON.
    let h1 = cache.intern_ref(packet.as_slice()).unwrap();

    // Second time: Cache hit. Returns handle immediately. 
    // ZERO allocation. ZERO JSON parsing overhead.
    let h2 = cache.intern_ref(packet.as_slice()).unwrap();

    assert_eq!(h1, h2);
}

```

### 3. Advanced: The "Rich Symbol" (Computed Metadata)

**Scenario:** You are building a compiler or analyzer. You want to intern identifiers, but you also want to know properties about them (e.g., "Is this a keyword?", "What is its hash?").

**The Problem:** `&str` -> `ToOwned` returns `String`. It cannot return a `Symbol` struct.

**The Solution:** Use `FromRef` to compute metadata *during* interning.

```rust
use std::{borrow::Borrow, hash::{Hash, Hasher}};

use xgx_intern::{Interner, FromRef};

#[derive(Debug, Clone, Eq, PartialEq)]
struct Symbol {
    text: String,
    // Metadata computed once at creation
    is_keyword: bool,
    length_score: usize, 
}

impl Hash for Symbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

impl Borrow<str> for Symbol {
    fn borrow(&self) -> &str {
        &self.text
    }
}

// Transform &str -> Symbol lazily
impl FromRef<str> for Symbol {
    fn from_ref(s: &str) -> Self {
        // Perform analysis here
        Symbol {
            text: s.to_string(),
            is_keyword: matches!(s, "if" | "while" | "fn" | "return"),
            length_score: s.len() * 2,
        }
    }
}

fn main() {
    let mut pool = Interner::<Symbol, _>::new(std::collections::hash_map::RandomState::new());

    // We look up using a simple string slice.
    let handle = pool.intern_ref("while").unwrap();
    
    // We get back a fully analyzed struct.
    let sym = pool.resolve(handle).unwrap();
    
    assert_eq!(sym.is_keyword, true);
}

```

## ⚙️ Customization

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

println!("Interned with ahash and got handle: {handle:?}");
```

You can see more rust hash benchmarks here: [Rust Hash Benchmarks](https://github.com/ogxd/gxhash?tab=readme-ov-file#benchmarks). Please make sure you understand the security and safety characteristics of your use case and your chosen algorithm before using it.

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

## ⚖️ License

This project is licensed under the ([LICENSE-MIT](https://spdx.org/licenses/MIT)).
