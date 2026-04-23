# RestSend

RestSend is a full-featured instant messaging project with backend services, an admin console, a Rust client core, and WASM bindings.

## Crates

- `crates/restsend-backend`: backend service with API, WebSocket, OpenAPI, and Admin
- `crates/restsend`: Rust client core
- `crates/restsend-wasm`: WebAssembly bindings
- `crates/restsend-macros`: internal macros

## Quick Start

```bash
git clone https://github.com/restsend/restsend-rs.git
cd restsend-rs
cargo check
```

## Run Backend

Build:

```bash
cargo build -p restsend-backend --release
```

Minimal `.env`:

```env
RS_ADDR=0.0.0.0:8080
RS_DATABASE_URL=sqlite://restsend-server.db?mode=rwc
RS_OPENAPI_PREFIX=/open
RS_API_PREFIX=/api
RS_LOG_FILE=logs/restsend-backend.log
RS_RUN_MIGRATIONS=true
```

Start:

```bash
cargo run -p restsend-backend --release
```

Override listen address by CLI:

```bash
cargo run -p restsend-backend --release -- --addr 127.0.0.1:18080
```

Health check:

```text
GET /api/health
```

## Admin

- Admin page: `/admin`
- First visit can bootstrap the first superuser
- After bootstrap, login with the superuser account

## Notes

- `.env` is loaded via `dotenvy`
- Default database is SQLite
- `RS_PRESENCE_BACKEND=memory` for single node
- `RS_PRESENCE_BACKEND=db` for shared presence across nodes

## Features

- REST API and WebSocket realtime messaging
- Admin console
- OpenAPI integration
- Webhook support
- SQLite / MySQL via SeaORM
- Rust and WASM client support

## License

- MIT, see `LICENSE`
- Commercial license required for production deployments serving more than 1000 users

Contact: `kui@fourz.cn`
