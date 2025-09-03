## xgx_intern

A simple Hash trait interner for rust.

### Supported Stored Types

Anything that supports the `Eq` and `Hash` rust std traits.

### Supported Index Types

Anything that can be converted into a usize. At the time of writing that is:

- `u128`
- `u64`
- `u32`
- `u16`
- `u8`
- `usize`

- `i128`
- `i64`
- `i32`
- `i16`
- `i8`
- `isize`

Remember to choose a type appropriate for your system's architecture:

- You may not use an index type greater than what your system architecture supports. (e.g. a 64 bit system will not support a u128 or i128 index). If you have the memory to support greater than your environment's architecture, consider an overflow mechanism into multiple Interners.
- Do not use `usize` or `isize` if you require compatibility between architectures. (e.g. 64 bit server communicating with a 32 bit wasm client)
