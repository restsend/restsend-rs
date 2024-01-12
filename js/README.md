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
    import * as restsend_wasm from 'restsend_wasm.js';
    restsend_wasm.then(m => {
        const info = m.signin(endpoint, userId, password)
        let client = new m.Client(endpoint, userId, info.token)
        ...
    })
    ```
