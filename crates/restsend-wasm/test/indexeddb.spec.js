import { describe, it, expect, assert, vi } from 'vitest'
const { Client, signin, setLogging } = await import('../pkg/restsend_wasm.js')
import { waitUntil, authClient, endpoint } from './common.js'
setLogging('info')

describe('Indexeddb performance', async function () {
    it('#store large logs - wasm', async () => {
        let allitems = []
        for (let j = 0; j < 20; j++) {
            let topicId = 'not_use_' + '*'.repeat(24) + j
            let items = []
            for(let i = 1; i <= 300; i++) {
                items.push({
                    topicId,
                    id: 'mock_id_' + '*'.repeat(24) + i,
                    seq: i,
                    senderId: 'vitalik' + '*'.repeat(10),
                    content: {type:'text', text: 'hello'.repeat(100)},
                    createdAt: new Date().toISOString(),
                    read:false,
                    recall:false,
                })
            }
            allitems.push(items)
        }

        let vitalikNotCache = await authClient('vitalik', 'vitalik:demo', false, 'vitalik-not-cache')
        {
            let st = new Date().getTime()
            for (let i = 1; i < allitems.length; i++) {
                await vitalikNotCache.saveChatLogs(allitems[i])
            }
            //console.log(`init avg cost ${(new Date().getTime() - st)/(allitems.length - 1)} ms`)
        }
        const topicId = allitems[0][0].topicId
        {
            await vitalikNotCache.saveChatLogs(allitems[0])
        }
        let logsCount = 0
        let syncCount = 0
        let result = []
        let st = new Date().getTime()
        let syncLogs = async () => {
            await vitalikNotCache.syncChatLogs(topicId, undefined, {
                limit: 100,
                onsuccess: (r) => {
                    if (r.items) {
                        logsCount += r.items.length
                        Array.from(r.items).forEach((item) => {
                            result.push(item)
                        })
                    }
                    syncCount += r.items.length
                }
            })
        }
        await syncLogs()
        expect(syncCount).toEqual(100)
        result.forEach((item, i) => {
            expect(item.topicId).toEqual(topicId)
        })
        //console.log(`sync cost ${(new Date().getTime() - st)} ms`)
    })
})