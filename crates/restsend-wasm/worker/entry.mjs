import { Client } from '../pkg/restsend_wasm.js'

let client = null

function sendResult(id, ok, data, error) {
    self.postMessage({ id, ok, data, error })
}

self.onmessage = async (event) => {
    const { id, method, args } = event.data || {}

    try {
        if (method === 'init') {
            const [info, dbName] = args || []
            client = new Client(info, dbName)
            sendResult(id, true, { inited: true }, null)
            return
        }

        if (!client) {
            throw new Error('worker client not initialized')
        }

        if (method === 'syncChatLogs') {
            const [topicId, lastSeq, option] = args || []
            let result = null
            let failReason = null

            await client.syncChatLogs(topicId, lastSeq, {
                limit: option?.limit,
                ensureConversationVersion: option?.ensureConversationVersion,
                heavy: option?.heavy,
                onsuccess: (r) => {
                    result = r
                },
                onfail: (e) => {
                    failReason = e
                },
            })

            if (failReason) {
                throw new Error(typeof failReason === 'string' ? failReason : JSON.stringify(failReason))
            }

            sendResult(id, true, result, null)
            return
        }

        if (method === 'shutdown') {
            await client.shutdown()
            sendResult(id, true, { shutdown: true }, null)
            return
        }

        throw new Error(`unsupported method: ${method}`)
    } catch (e) {
        sendResult(id, false, null, e && e.message ? e.message : String(e))
    }
}
