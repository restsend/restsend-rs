const { defineConfig } = require('vite')
import wasm from "vite-plugin-wasm";
const demoServer = 'https://chat.ruzhila.cn/'

export default defineConfig({
    plugins: [
        wasm()
    ],
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