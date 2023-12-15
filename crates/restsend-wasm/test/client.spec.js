import { describe, it, expect, assert } from 'vitest'
const { Client, signup, signin, enable_logging } = await import('../pkg/restsend_wasm.js');

const endpoint = 'https://chat.ruzhila.cn/'

async function waitUntil(fn, timeout) {
    let start = Date.now()
    while (true) {
        if (fn()) {
            return true
        }
        if (Date.now() - start > timeout) {
            return false
        }
        await new Promise(resolve => setTimeout(resolve, 100))
    }
}

enable_logging('info')

async function authClient(username, password, withWebSocket = false) {
    let info = await signin(endpoint, username, password)
    let client = new Client(endpoint, username, info.token)

    if (withWebSocket) {
        await client.connect()
        await waitUntil(() => client.connection_status === 'connected', 3000)
    }
    return client
}

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

describe('Websocket connection', function () {
    it('#setup callback', async () => {
        let info = await signin(endpoint, 'guido', 'guido:demo')
        let client = new Client(endpoint, 'guido', info.token)
        let is_connected = false
        client.onconnected = () => {
            is_connected = true
        }
        await client.connect()
        await waitUntil(() => client.connection_status === 'connected', 3000)
        expect(is_connected).toBe(true)
    })
    it('#handshake', async () => {
        let guido = await authClient('guido', 'guido:demo', true)
        expect(guido.connection_status).toBe('connected')
    })
    it('#send text message', async () => {
        let bob = await authClient('bob', 'bob:demo', true)
        expect(bob.connection_status).toBe('connected')
        let is_sent = false
        let is_ack = false
        let is_fail = false
        bob.doSendText('bob:alice', 'hello', {
            onsent: () => {
                is_sent = true
            },
            onack: (req) => {
                is_ack = true
            },
            onerror: (e) => {
                is_fail = true
            }
        })
        await waitUntil(() => is_ack, 3000)
        expect(is_fail).toBe(false)
        expect(is_ack).toBe(true)
        expect(is_sent).toBe(true)
    })

    it('#send image message', async () => {
        let bob = await authClient('bob', 'bob:demo', true)
        let is_ack = false

        bob.doSendImage('bob:alice', {
            'url': 'https://avatars1.githubusercontent.com/u/1016365?s=460&v=4',
        }, {
            onack: (req) => {
                is_ack = true
            },
        })
        await waitUntil(() => is_ack, 3000)

        is_ack = false
        bob.doSendImage('bob:alice', {
            'file': new File(['(⌐□_□)'], 'hello_restsend.png', { type: 'image/png' }),
        }, {
            onack: (req) => {
                is_ack = true
            },
        })
        await waitUntil(() => is_ack, 3000)
    })
})