### How intgrate with your project
1. Test rust library, the outout dir is `restsend-rs/crates/restsend-wasm/pkg`
    ```shell
    cd crates/restsend-wasm
    npm run test
    ```
2. Build rust library, the outout dir is `restsend-rs/js`
    ```shell
    cd crates/restsend-wasm
    npm run dist
    ```
3. import `restsend_wasm.js` in your project
    ```javascript
    import restsendWasm from 'restsend_wasm.js';
    restsendWasm().then(m => {
        m.signin(endpoint, userId, password).then(info => {
            let client = new m.Client(info)
        })
    })
    ```

### Worker mode (recommended for heavy log sync)

This package now ships a ready-to-use Worker helper, so developers do not need to write
their own `postMessage` RPC plumbing.

```javascript
import { signin } from 'resetsend-sdk'
import { createWorkerClient } from 'resetsend-sdk/worker'

const info = await signin(endpoint, userId, password)
const client = await createWorkerClient(info, 'my-db')

// Same API shape as Client. syncChatLogs is routed to worker by default.
await client.syncChatLogs(topicId, undefined, {
    limit: 20,
    onsuccess: (r) => console.log(r.items.length),
})

await client.shutdown()
```

Notes:

1. Worker helper auto-falls back to main-thread client if worker init fails.
2. You can disable worker for a single call: `option.worker = false`.
3. Advanced: pass a custom worker URL in `createWorkerClient(info, dbName, { workerUrl })`.
