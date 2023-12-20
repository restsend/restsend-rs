import { describe, it, expect } from 'vitest'
const { Client, signin, setLogging } = await import('../pkg/restsend_wasm.js')
import { waitUntil, authClient, endpoint } from './common.js'

setLogging('info')
describe('Messages', async function () {
    it('#setup callback', async () => {
        let info = await signin(endpoint, 'guido', 'guido:demo')
        let client = new Client(info)
        let is_connected = false
        client.onconnected = () => {
            is_connected = true
        }
        await client.connect()
        await waitUntil(() => client.connectionStatus === 'connected', 3000)
        expect(is_connected).toBe(true)
    })
    let bob = await authClient('bob', 'bob:demo', true)
    it('#websocket handshake', async () => {
        let guido = await authClient('guido', 'guido:demo', true)
        expect(guido.connectionStatus).toBe('connected')
    })
    bob.onconversationsupdated = async (items) => {
        console.log('onconversationsupdated', items)
    }

    it('#send text message', async () => {
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
        let is_ack = false
        bob.doSendImage('bob:alice', {
            'url': 'https://avatars1.githubusercontent.com/u/1016365?s=460&v=4',
        }, {
            onack: (req) => {
                is_ack = true
            },
        })
        await waitUntil(() => is_ack, 3000)
    })

    it('#send image message with upload', async () => {
        let is_ack = false
        is_ack = false
        bob.doSendImage('bob:alice', {
            'file': new File(['xxx'], 'hello_restsend.png', { type: 'image/png' }),
        }, {
            onack: (req) => {
                is_ack = true
            },
        })
        await waitUntil(() => is_ack, 3000)
    })
    it('#send custom content', async () => {
        let is_ack = false
        is_ack = false
        await bob.doSend(
            'bob:alice',
            {
                'type': 'custom',
                'text': JSON.stringify({
                    'type': 'text',
                    'text': 'hello'
                })
            },
            {
                onack: (req) => {
                    is_ack = true
                },
            });
        await waitUntil(() => is_ack, 3000)
        expect(is_ack).toBe(true)
    })
    // it('#send custom content with upload', async () => {
    //     let is_ack = false
    //     is_ack = false
    //     await bob.doSend(
    //         'bob:alice',
    //         {
    //             type: 'custom.image',
    //             attachment:
    //             {
    //                 'file': new File(['custom image'], 'hello_custom.png', { type: 'image/png' }),
    //             },
    //         },
    //         {
    //             onack: (req) => {
    //                 is_ack = true
    //             },
    //         });
    //     await waitUntil(() => is_ack, 3000)
    //     expect(is_ack).toBe(true)
    // })
})