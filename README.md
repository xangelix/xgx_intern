## xgx_intern

A simple Hash trait interner for rust.

## Why u32?

For proper wasm32 support.

wasm32 is a relevant target for many, and wasm64 support is still very poor in the rust ecosystem. Expect to convert a lot between u32 and usize.
