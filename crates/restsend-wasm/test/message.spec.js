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
            'url': 'https://ruzhila.cn/_nuxt/golang.c9f726ce.jpg',
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
        // an simple png image base64 
        let canvas = document.createElement('canvas');
        canvas.width = 200;
        canvas.height = 200;

        let ctx = canvas.getContext('2d');
        ctx.fillStyle = 'lightblue';
        ctx.fillRect(0, 0, canvas.width, canvas.height);

        let now = new Date();
        let seconds = now.getSeconds();
        let minutes = now.getMinutes();
        let hours = now.getHours();

        ctx.beginPath();
        ctx.arc(100, 100, 80, 0, 2 * Math.PI);
        ctx.stroke();

        for (let i = 0; i < 12; i++) {
            ctx.beginPath();
            ctx.moveTo(100 + 70 * Math.sin(i / 6 * Math.PI), 100 - 70 * Math.cos(i / 6 * Math.PI));
            ctx.lineTo(100 + 80 * Math.sin(i / 6 * Math.PI), 100 - 80 * Math.cos(i / 6 * Math.PI));
            ctx.stroke();
        }

        ctx.beginPath();
        ctx.moveTo(100, 100);
        ctx.lineTo(100 + 40 * Math.sin(hours / 6 * Math.PI), 100 - 40 * Math.cos(hours / 6 * Math.PI));
        ctx.stroke();

        ctx.beginPath();
        ctx.moveTo(100, 100);
        ctx.lineTo(100 + 60 * Math.sin(minutes / 30 * Math.PI), 100 - 60 * Math.cos(minutes / 30 * Math.PI));
        ctx.stroke();

        ctx.beginPath();
        ctx.moveTo(100, 100);
        ctx.lineTo(100 + 70 * Math.sin(seconds / 30 * Math.PI), 100 - 70 * Math.cos(seconds / 30 * Math.PI));
        ctx.stroke();
        let pngData = atob(canvas.toDataURL('image/png').split(',')[1]);
        let array = [];
        for (let i = 0; i < pngData.length; i++) {
            array.push(pngData.charCodeAt(i));
        }
        let blob = new Blob([new Uint8Array(array)], { type: 'image/png' });
        bob.doSendImage('bob:alice', {
            'file': new File([blob], 'hello_restsend.png', { type: 'image/png' }),
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
        expect(sentContent.size).toBe(array.length)
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