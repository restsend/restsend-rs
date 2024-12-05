# restsend_dart

## How to use

### Install rust
```shell
export RUSTUP_DIST_SERVER="https://rsproxy.cn"
export RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
curl --proto '=https' --tlsv1.2 -sSf https://rsproxy.cn/rustup-init.sh | sh
```

### Use rsproxy.cn mirror
Change `~/.cargo/config.toml` to:
```toml
[source.crates-io]
replace-with = 'rsproxy-sparse'
[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"
[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"
[registries.rsproxy]
index = "https://rsproxy.cn/crates.io-index"
[net]
git-fetch-with-cli = true
```

### Install flutter_rust_bridge_codegen
```shell
cargo install flutter_rust_bridge_codegen
```

### Generate the bridge code
> Optional, only if you want to regenerate the bridge code.
> 

In the root of the project, run:

```shell 
flutter_rust_bridge_codegen  generate --no-web
```