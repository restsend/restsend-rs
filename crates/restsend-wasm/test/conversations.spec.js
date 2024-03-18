import { describe, it, expect, assert } from 'vitest'
const { Client, signin, setLogging } = await import('../pkg/restsend_wasm.js')
import { waitUntil, authClient, endpoint } from './common.js'

describe('Conversations', async function () {
    let vitalik = await authClient('vitalik', 'vitalik:demo', false)
    let guidoTopic = undefined
    it('#create conversation', async () => {
        await vitalik.createChat('alice')
        await vitalik.createChat('bob')
        guidoTopic = await vitalik.createChat('guido')
    })

    it('#set tags, extra', async () => {
        await vitalik.setConversationTags(guidoTopic.topicId, [{ id: 'tag1' }, { id: 'tag2' }])
        let r = await vitalik.setConversationExtra(guidoTopic.topicId, { "key1": "value1" })
        expect(r).toHaveProperty('tags')
        expect(r.tags).toEqual([{ id: 'tag1' }, { id: 'tag2' }])
        expect(r.extra).toStrictEqual({ key1: 'value1' })
    })

    it('#sync conversation', async () => {
        let cbCount = 0
        let conversations = []

        vitalik.onconversationsupdated = async (items) => {
            conversations.push(...items)
        }

        await vitalik.syncConversations({
            onsuccess(updatedAt, count) {
                cbCount += count
            }
        })

        await waitUntil(() => cbCount > 0, 3000)
        expect(cbCount).toBeGreaterThan(3)
        expect(conversations.length).toEqual(cbCount)
    })

    it('#sync sync last logs', async () => {
        await vitalik.connect()
        await waitUntil(() => vitalik.connectionStatus === 'connected', 3000)

        let ackCount = 0
        let sendCount = 10;
        let sendIds = []
        for (let i = 0; i < sendCount; i++) {
            let id = await vitalik.doSendText(guidoTopic.topicId, `hello guido ${i}`, {
                onack: () => {
                    ackCount += 1;
                },
                onfail: (reason) => {
                    assert.fail(reason)
                }
            })
            sendIds.unshift(id)
        }

        await waitUntil(() => ackCount == sendCount, 10000)
        expect(ackCount).toEqual(sendCount)
        expect(sendIds.length).toEqual(sendCount)

        let logsCount = 0
        let syncMax = 100
        let items = []
        let syncCount = 0
        let syncLogs = async () => {
            await vitalik.syncChatLogs(guidoTopic.topicId, undefined, {
                limit: 10,
                onsuccess: (r) => {
                    if (r.items) {
                        logsCount += r.items.length
                    }
                    r.items.forEach((item) => {
                        items.push(item.id)
                    })

                    if (!r.hasMore || logsCount >= syncMax) {
                        return
                    }
                    syncCount += 1
                    if (syncCount > 10) {
                        assert.fail('syncCount > 10')
                    }
                    setTimeout(() => {
                        syncLogs().then()
                    }, 0)
                }
            })
        }
        await syncLogs()
        await waitUntil(() => logsCount >= syncMax, 3000)
        expect(logsCount).toEqual(syncMax)

        expect(items.length).toEqual(syncMax)
        expect(items.slice(0, sendIds.length)).toEqual(sendIds)

        let recallAck = false
        let recallId = sendIds[0]
        let recallSeq = 0
        await vitalik.doRecall(guidoTopic.topicId, recallId, {
            onack: (req) => {
                recallAck = true
                recallSeq = req.seq
            }
        })
        await waitUntil(() => recallAck, 3000)
        expect(recallAck).toBe(true)

        let syncDone = false
        let newItems = []
        await vitalik.syncChatLogs(guidoTopic.topicId, recallSeq, {
            limit: 10,
            onsuccess: (r) => {
                syncDone = true
                newItems = r.items
            }
        })
        await waitUntil(() => syncDone, 3000)
        expect(syncDone).toBe(true)
        expect(newItems[0].content.type).toEqual('recall')
        expect(newItems[0].content.text).toEqual(recallId)
        expect(newItems[1].recall).toBe(false)
        expect(newItems[1].content.type).toEqual('text')
    })

    it('#get conversation last message', async () => {
        let lastMessage = undefined
        vitalik.onconversationsupdated = async (items) => {
            items.filter((c) => c.topicId === guidoTopic.topicId).forEach((c) => {
                lastMessage = c.lastMessage
            })
        }
        let syncDone = false
        await vitalik.syncConversations({
            onsuccess(updatedAt, count) {
                syncDone = true
            },
        })
        await waitUntil(() => syncDone, 3000)
        expect(syncDone).toBe(true)
        expect(lastMessage.type).toEqual('')

        vitalik.onconversationsupdated = async (items) => {
        }

        let isAck = false
        let updateExtraId = await vitalik.doSendText(guidoTopic.topicId, `hello guido will update extra`, {
            onack: () => {
                isAck = true
            },
            onfail: (reason) => {
                assert.fail(reason)
            }
        })

        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)

        await vitalik.doUpdateExtra(guidoTopic.topicId, updateExtraId, { 'tag is': 'me' }, {
            onack: (req) => {
                isAck = true
            },
        })
        await waitUntil(() => isAck, 3000)
        expect(isAck).toBe(true)

        vitalik.onconversationsupdated = async (items) => {
            items.filter((c) => c.topicId === guidoTopic.topicId).forEach((c) => {
                lastMessage = c.lastMessage
                syncDone = true
            })
        }
        syncDone = false
        await vitalik.syncConversations({
            onsuccess(updatedAt, count) {
            },
        })
        await waitUntil(() => syncDone, 3000)
        expect(syncDone).toBe(true)
        expect(lastMessage.type).toEqual('text')
        console.log('lastMessage', lastMessage)
        expect(lastMessage.extra).toEqual({ 'tag is': 'me' })
    })

    it('#filter conversations', async () => {
        let conversations = await vitalik.filterConversation(c => {
            return c.attendee === 'guido'
        })
        expect(conversations.length).toEqual(1)
        expect(conversations[0].attendee).toEqual('guido')
    })
})