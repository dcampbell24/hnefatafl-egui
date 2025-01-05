This is a basic Hnefatafl app designed to demonstrate what you can build with the
[`hnefatafl-rs`](https://crates.io/crates/hnefatafl) crate in Rust. You can play a few different variants against a basic (not very strong) AI.
It uses the [`egui`](https://crates.io/crates/egui) GUI library and can be built as a native or web app. You can see the web version at
https://bunburya.eu/hnefatafl-egui/

# Building

First, clone the git repo and navigate to the cloned directory.

```shell
git clone https://github.com/bunburya/hnefatafl-egui.git
cd hnefatafl-egui
```

## Native

Building and running `hnefatafl-egui` as a native app should be as easy as:

```shell
cargo build --release
```

You will then find the `hnefatafl-egui` executable in the `target/release/` directory.

## Web

Because we use are using threads (via the `wasm-thread` crate), we need to use nightly Rust. Build with:

```shell
RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals" cargo build --target=wasm32-unknown-unknown --release -Z build-std=panic_abort,std
```

Install the `wasm-bindgen` command-line interface if you have not done so:

```shell
cargo install wasm-bindgen-cli
```

Then generate the WASM bindings (placing them in the `srv/pkg/` directory) with:

```shell
wasm-bindgen --target web --out-dir srv/pkg target/wasm32-unknown-unknown/release/hnefatafl_egui.wasm
```

And then serve the files in the `srv/` directory using the web server of your choice. You must send the
`Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` headers in your response. 
There is a very basic Python server included in the `scripts/` directory that does just that. You can run it like so:
```shell
cd srv
python3 ../scripts/serve.py
```
This can be useful for testing purposes, **but it should not be used in production**.

# Contributing

This app is really just intended as a basic demo so I will likely not be putting much further effort into enhancing it
(unless I want to use it to showcase additional `hnefatafl-rs` capabilities in the future). If you encounter any major
problems, feel free to create an issue. Pull requests to fix issues or enhance the AI or UI are also welcome.
