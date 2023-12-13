/**
 * unittest for client.js
 */
import WebSocket from "ws";
const { Client, signup, signin } = await import('../pkg/restsend_wasm.js');

import { describe, it, expect, assert, vi } from 'vitest'
import { loadEnv } from 'vite'

const env = loadEnv('development', process.cwd())
const endpoint = env.VITE_API_ENDPOINT || 'https://chat.ruzhila.cn/'

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

function createWebsocketWithToken(url, token) {
    return new WebSocket(url + "&unittest=true", {
        finishRequest(req, ws) {
            req.on('socket', (socket) => {
                socket.on('connect', () => {
                    req.setHeader('Authorization', `Bearer ${token}`);

                    req.end();
                })
            })
        },
    })
}

async function authClient(username, password, withWebSocket = false) {
    let info = await signin(endpoint, username, password)
    let client = new Client(endpoint, username, info.token)

    if (withWebSocket) {
        // client.newWebSocket = (url) => {
        //     console.log('new websocket', url)
        //     return createWebsocketWithToken(url, client.token)
        // }
        await client.connect()
        await waitUntil(() => client.status === 'connected', 3000)
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

    describe('#handshake', async () => {
        let guido = await authClient('guido', 'guido:demo', true)
    })
})