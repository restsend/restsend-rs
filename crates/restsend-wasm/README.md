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
