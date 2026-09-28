# RestSend ArkTS Client (HarmonyOS)

ArkTS port of the rust client core (`crates/restsend`): a HAR SDK
(`Library/restsend`) plus a demo app (`entry`). The sdk core is
platform-neutral; `@ohos` transport/storage adapters are injected.

```
ohos/
├── Library/restsend/   # HAR sdk: models, ws/http protocol, sync, storage
├── entry/              # demo app: login / conversations / chat
├── harness/            # node runtime: run the sdk core against a real backend
└── scripts/            # linux ci scripts (setup / build / e2e)
```

## Build & test (linux, no DevEco Studio needed)

One command (first run downloads the commandline-tools, ~2 GB):

```bash
cd ohos && ./verify.sh
```

What it does: toolchain setup -> `assembleHar` + `assembleHap` ->
start a local `restsend-backend` (cargo) -> run 17 e2e tests.

Individual steps:

```bash
./scripts/setup-linux-env.sh        # install/check the toolchain
./scripts/build.sh                  # ohpm install, assembleHar + assembleHap
./scripts/e2e.sh                    # build backend, run harness tests
./harness/soak.mjs 60               # long-run leak check (optional)
```

Requirements: rust (cargo), node >= 20, linux x64. The toolchain is the
official HarmonyOS commandline-tools (hvigor + sdk, API 21).

## Backend for tests

```bash
cargo build -p restsend-backend
HOST=0.0.0.0 ./harness/backend.sh   # 127.0.0.1:18123 by default
```

## Device / emulator

Open `ohos/` in DevEco Studio, pick an emulator or device and Run
(auto-signed). Point the app at the LAN IP of the machine running the
backend. Signing needs Huawei certificates; emulators run unsigned HAPs.
Check on device: ws bearer header, multipart upload, RDB, background
keepalive.

## API

The exported surface mirrors the rust `ClientInterface` (auth, connection,
senders, sync, topics, users, attachments, errors). See
`Library/restsend/src/main/ets/Index.ets` for exports and
`harness/test.mjs` for behavior examples against a real backend.
