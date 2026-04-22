import { Client } from '../pkg/restsend_wasm.js'

let client = null

function postResult(id, ok, data, error) {
    self.postMessage({ id, ok, data, error })
}

self.onmessage = async (event) => {
    const { id, method, args } = event.data || {}
    try {
        if (method === 'init') {
            const [info, dbName] = args
            client = new Client(info, dbName)
            postResult(id, true, { inited: true })
            return
        }

        if (!client) {
            throw new Error('worker client not initialized')
        }

        if (method === 'syncChatLogs') {
            const [topicId, lastSeq, option] = args
            let successResult = null
            let failReason = null

            await client.syncChatLogs(topicId, lastSeq, {
                limit: option?.limit,
                onsuccess: (r) => {
                    successResult = r
                },
                onfail: (e) => {
                    failReason = e
                },
            })

            if (failReason) {
                throw new Error(typeof failReason === 'string' ? failReason : JSON.stringify(failReason))
            }

            postResult(id, true, successResult)
            return
        }

        if (method === 'shutdown') {
            await client.shutdown()
            postResult(id, true, { shutdown: true })
            return
        }

        throw new Error(`unsupported method: ${method}`)
    } catch (e) {
        postResult(
            id,
            false,
            null,
            e && e.message ? e.message : String(e)
        )
    }
}
