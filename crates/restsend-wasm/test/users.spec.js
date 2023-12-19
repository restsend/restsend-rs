import { describe, it, expect } from 'vitest'
import { authClient } from './common.js'

describe('Users', function () {
    describe('#get user info', async function () {
        let bob = await authClient('bob', 'bob:demo', false)
        it('should get user info without blocking', async () => {
            let bobInfo = await bob.getUser('bob')
            expect(bobInfo.isPartial).toBe(true)
        })
        it('should get user info with blocking', async () => {
            let bobInfo = await bob.getUser('bob', true)
            expect(bobInfo.isPartial).toBe(false)
            expect(bobInfo).toHaveProperty('avatar')
        })
        it('should get user info with blocking', async () => {
            let alice = await bob.getUser('alice', true)
            console.log(alice)
            expect(alice.isPartial).toBe(false)
            expect(alice).toHaveProperty('avatar')
        })

        it('should get users', async () => {
            let users = await bob.getUsers(['bob', 'alice', 'guido', 'bad_guy'])
            expect(users.length).toBe(4)
            let badGuy = users.find(u => u.userId === 'bad_guy')
            expect(badGuy.isPartial).toBe(true)
        })
    })
})
