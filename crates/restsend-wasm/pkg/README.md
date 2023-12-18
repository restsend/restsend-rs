### How intgrate with your project
1. Build rust library
    ```shell
    cd crates/restsend-wasm
    npm run build
    ```
2. Copy `pkg/restsend_wasm.js` and `pkg/restsend_wasm_bg.wasm` to your project
3. Import `restsend_wasm.js` in your project
    ```javascript
    import * as restsend_wasm from './restsend_wasm.js';
    restsend_wasm.then(m => {
        const info = m.signin(endpoint, userId, password)
        let client = new m.Client(endpoint, userId, info.token)
        ...
    })
    ```
