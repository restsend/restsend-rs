import { resolve } from 'path'
const { defineConfig } = require('vite')
import wasm from "vite-plugin-wasm";

export default defineConfig({
    plugins: [
        wasm()
    ],

    build: {
        target: 'esnext',
        outDir: '../../js',
        rollupOptions: {
            input: {
                "restsend_wasm": resolve(__dirname, 'pkg/restsend_wasm.js'),
            },
            output: {
                entryFileNames: '[name].js',
                chunkFileNames: '[name].js',
                assetFileNames: '[name].[ext]',
            },
        },
    },
    server: {
        proxy: {
            '/api/connect': {
                target: 'https://chat.ruzhila.cn',
                changeOrigin: true,
                ws: true,
            },
            '/auth': {
                target: 'https://chat.ruzhila.cn',
                changeOrigin: true,
            },
            '/api': {
                target: 'https://chat.ruzhila.cn',
                changeOrigin: true,
            },
        }
    },
    optimizeDeps: { exclude: ["fsevents"] },
    test: {
        
        browser: {
            provider: 'playwright',
            enabled: true,
            headless: true,
            name: 'webkit', // browser name is required
        },
        testTimeout: 20000, // 20 seconds
    },
});