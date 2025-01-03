

## Native

Building and running `hnefatafl-egui` as a native app should be as easy as:

```shell
cargo build --release
```

You will then find the `hnefatafl-egui` executable in the `target/release/` directory.

## Web

Because we use are using threads (via the `wasm-thread` crate), we need to use nightly Rust. Build with

```shell
RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals" cargo build --target=wasm32-unknown-unknown --release -Z build-std=panic_abort,std
```

Then generate the WASM bindings with:

```shell
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/hnefatafl_egui.wasm
```

And then serve `index.html` using the server of your choice. You must send the `Cross-Origin-Opener-Policy: same-origin`
and `Cross-Origin-Embedder-Policy: require-corp` headers in your response. There is a very basic Python server included
in the `scripts/` directory that does just that (`python3 scripts/serve.py`). This can be useful for testing purposes,
**but it should not be used in production**.
