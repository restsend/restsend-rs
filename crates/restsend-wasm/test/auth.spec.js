import { describe, it, expect, assert } from 'vitest'
const { Client, signup, signin } = await import('../pkg/restsend_wasm.js');
import { endpoint } from './common.js'

describe('Client auth', function () {
    describe('#constructor', function () {
        it('should create a client instance', function () {
            var client = new Client('endpoint_value', 'user_id_value', 'token_value')
            assert.ok(client)
        })
    });
    describe('#endpoint status', function () {
        it('test endpoint is running', async function () {
            var resp = await fetch(`${endpoint}api/connect`)
            expect(resp.status).toBe(401)
        });
        it('prepare unittest accounts', async function () {
            await signup(endpoint, 'guido', 'guido:demo').catch(e => { })
            await signup(endpoint, 'vitalik', 'vitalik:demo').catch(e => { })
            await signup(endpoint, 'alice', 'alice:demo').catch(e => { })
            await signup(endpoint, 'bob', 'bob:demo').catch(e => { })
        });
    });
    describe('#login', function () {
        it('should login', async () => {
            expect(await signin(endpoint, 'guido', 'guido:demo')).toHaveProperty('token')
            expect(await signin(endpoint, 'vitalik', 'vitalik:demo')).toHaveProperty('token')
            expect(await signin(endpoint, 'alice', 'alice:demo')).toHaveProperty('token')
            expect(await signin(endpoint, 'bob', 'bob:demo')).toHaveProperty('token')
        })
    })
})