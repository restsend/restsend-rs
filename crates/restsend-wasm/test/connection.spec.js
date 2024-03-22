import { describe, it, expect, assert } from 'vitest'
import { waitUntil, authClient, } from './common.js'

describe('Connections', async function () {
    it('#connections shutdown', async () => {
        let user = ['alice', 'bob', 'vitalik']
        for (let i = 0; i < 10; i++) {
            let u = user[i % user.length]
            let conn = await authClient(u, u + ':demo', true)
            await conn.syncConversations({})
            await conn.createChat('alice')
            await conn.shutdown()
        }
    })
})