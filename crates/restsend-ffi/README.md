## How to generate the bindings
```bash

cd restsend-rs

cargo build --release
cargo run --bin bindgen generate --library target/release/librestsend_ffi.dylib --out-dir crates/restsend-ffi/bindings/python --language python
```