import { describe, it, expect } from 'vitest'
const { Client, signin, enable_logging } = await import('../pkg/restsend_wasm.js')
import { waitUntil, authClient, endpoint } from './common.js'

describe('Messages', function () {
    it('#sync conversation', async () => {
        let bob = await authClient('bob', 'bob:demo', false)
        let cb_count = 0
        let conversations = []

        bob.onconversationsupdated = async (items) => {
            conversations.push(...items)
        }

        await bob.syncConversations({
            onsuccess(updatedAt, count) {
                cb_count += count
            }
        })

        await waitUntil(() => cb_count > 0, 3000)
        expect(cb_count).toBeGreaterThan(0)
        expect(conversations.length).toEqual(cb_count)
    })
})