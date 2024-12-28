## Web

Because we use are using threads (via the `wasm-thread` crate), we need to use nightly Rust. Build with

```shell
RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals" cargo build --target=wasm32-unknown-unknown --release -Z build-std=panic_abort,std
```

Then generate the WASM bindings with:

```shell
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/tafl_egui.wasm
```

And then serve `index.html` using the server of your choice. You must send the `Cross-Origin-Opener-Policy` and `Cross-Origin-Embedder-Policy` headers in your response.
