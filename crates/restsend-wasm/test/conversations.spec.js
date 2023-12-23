import { describe, it, expect } from 'vitest'
const { Client, signin, setLogging } = await import('../pkg/restsend_wasm.js')
import { waitUntil, authClient, endpoint } from './common.js'

describe('Conversations', function () {
    it('#create conversation', async () => {
        setLogging('debug')
        let bob = await authClient('bob', 'bob:demo', false)
        await bob.createChat('alice')
        await bob.createChat('guido')
        await bob.createChat('vitalik')
    })

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
        console.log(conversations)
    })
})