const path = require('path')
const { defineConfig } = require('vite')
import wasm from "vite-plugin-wasm";

module.exports = defineConfig({
    plugins: [
        wasm(),
    ],
    build: {
        target: 'esnext',
        assetInlineLimit: 5 * 1024 * 1024,
        lib: {
            entry: path.resolve(__dirname, 'index.js'),
            name: 'restsend-sdk',
            fileName: (format) => `restsend-sdk.${format}.js`
        }
    }
});