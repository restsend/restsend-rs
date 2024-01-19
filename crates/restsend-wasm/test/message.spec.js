import { describe, it, expect } from 'vitest'
const { Client, signin, setLogging } = await import('../pkg/restsend_wasm.js')
import { waitUntil, authClient, endpoint } from './common.js'

describe('Messages', async function () {
    it('#setup callback', async () => {
        let info = await signin(endpoint, 'guido', 'guido:demo')
        let client = new Client(info, 'guido')
        let isConnected = false
        client.onconnected = () => {
            isConnected = true
        }
        await client.connect()
        await waitUntil(() => client.connectionStatus === 'connected', 3000)
        expect(isConnected).toBe(true)
    })
    let bob = await authClient('bob', 'bob:demo', true)
    it('#websocket handshake', async () => {
        let guido = await authClient('guido', 'guido:demo', true)
        expect(guido.connectionStatus).toBe('connected')
    })

    it('#send text message', async () => {
        let isSent = false
        let isAck = false
        let isFail = false
        bob.doSendText('bob:alice', 'hello', {
            onsent: () => {
                isSent = true
            },
            onack: (req) => {
                isAck = true
            },
            onfail: (e) => {
                isFail = true
            }
        })
        await waitUntil(() => isAck, 3000)
        expect(isFail).toBe(false)
        expect(isAck).toBe(true)
        expect(isSent).toBe(true)
    })

    it('#send image message', async () => {
        let isAck = false
        bob.doSendImage('bob:alice', {
            'url': 'https://sd.zaowuyun.com/asset/kalnd5p7kq8mjm4p.png.1024.jpg',
        }, {
            onack: (req) => {
                isAck = true
            },
        })
        await waitUntil(() => isAck, 3000)
    })

    it('#send image message with upload', async () => {
        let isAck = false
        let sentContent = undefined
        bob.doSendImage('bob:alice', {
            'file': new File(['xxx'], 'hello_restsend.png', { type: 'image/png' }),
        }, {
            onattachmentupload: (result) => {
                return {
                    type: 'image',
                    text: result.path,
                    size: result.size,
                    placeholder: result.fileName,
                }
            },
            onack: (req) => {
                isAck = true
                sentContent = req.content
            },
        })
        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)
        expect(sentContent).toHaveProperty('text')
        expect(sentContent.text).toContain('https://')
        expect(sentContent.placeholder).toBe('hello_restsend.png')
        expect(sentContent.size).toBe(3)
    })
    let lastSendId = undefined
    it('#send custom content', async () => {
        let isAck = false
        lastSendId = await bob.doSend(
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
                    isAck = true
                },
            });
        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)
    })
    it('#send custom content with reply', async () => {
        let isAck = false
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
                'reply': lastSendId,
                onack: (req) => {
                    expect(req.content.reply).toBe(lastSendId)
                    isAck = true
                },
            });
        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)
    })
    it('#send custom content with upload', async () => {
        let isAck = false
        let sentContent = undefined

        await bob.doSend(
            'bob:alice',
            {
                type: 'custom.image',
                attachment:
                {
                    'file': new File(['custom image'], 'hello_custom.png', { type: 'image/png' }),
                },
            },
            {
                onack: (req) => {
                    isAck = true
                    sentContent = req.content
                },
            });
        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)
        expect(sentContent).toHaveProperty('text')
        expect(sentContent.placeholder).toBe('hello_custom.png')
        expect(sentContent.size).toBe(12)
    })
    it('#send update extra', async () => {
        let isAck = false
        let id = await bob.doSendText(
            'bob:alice', 'Need to update extra',
            {
                onack: (req) => {
                    isAck = true
                },
            });
        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)

        isAck = false
        await bob.doUpdateExtra('bob:alice', id, {
            'foo': 'bar'
        }, {
            onack: (req) => {
                isAck = true
            },
        })

        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)
        isAck = false

        let items = []
        await bob.syncChatLogs('bob:alice', undefined, {
            limit: 2,
            onsuccess: (r) => {
                items = r.items
                isAck = true
            }
        })
        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)
        expect(items[1].content.extra).toStrictEqual({ foo: 'bar' })

        let bob2 = await authClient('bob', 'bob:demo')
        isAck = false
        items = []
        await bob2.syncChatLogs('bob:alice', undefined, {
            limit: 2,
            onsuccess: (r) => {
                items = r.items
                isAck = true
            }
        })
        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)
        expect(items[1].content.extra).toStrictEqual({ foo: 'bar' })
    })
    it('#send logs', async () => {
        let items = []
        let isAck = false
        await bob.syncChatLogs('bob:alice', undefined, {
            limit: 2,
            onsuccess: (r) => {
                items = r.items
                isAck = true
            }
        })
        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)
        let ids = items.map((item) => item.id)
        isAck = false
        let content = undefined
        let id = await bob.doSendLogs(
            'bob:alice', 'bob:alice', ids,
            {
                onack: (req) => {
                    isAck = true
                    content = req.content
                },
            });
        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)
        let r = await fetch(content.text)
        let logs = await r.json()
        expect(content.type).toBe('logs')
        expect(logs.logIds).toStrictEqual(ids)
    })
})