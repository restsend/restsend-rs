import { describe, it, expect } from 'vitest'
const { signin } = await import('../pkg/restsend_wasm.js')
const { createWorkerClient } = await import('../worker/client.mjs')
import { endpoint } from './common.js'

describe('Worker quick path', async function () {
    it('#sync chat logs in worker', async () => {
        const info = await signin(endpoint, 'vitalik', 'vitalik:demo')
        const workerClient = await createWorkerClient(info, '')

        const result = await workerClient.syncChatLogs('vitalik:guido', undefined, { limit: 5 })

        expect(result).toBeTruthy()
        expect(Array.isArray(result.items)).toBe(true)
        expect(result.items.length).toBeGreaterThan(0)

        await workerClient.shutdown()
    })
})
