## How to generate the bindings
```bash

cd restsend-rs

cargo build --release
cargo run --bin bindgen -- --language python
```