import { Client } from '../pkg/restsend_wasm.js'

class WorkerRpc {
    constructor(worker, timeoutMs) {
        this.worker = worker
        this.timeoutMs = timeoutMs
        this.seq = 0
        this.pending = new Map()

        this.worker.onmessage = (event) => {
            const { id, ok, data, error } = event.data || {}
            const resolver = this.pending.get(id)
            if (!resolver) {
                return
            }
            this.pending.delete(id)
            if (ok) {
                resolver.resolve(data)
            } else {
                resolver.reject(new Error(error || 'worker call failed'))
            }
        }

        this.worker.onerror = (event) => {
            const err = new Error(event?.message || 'worker runtime error')
            for (const [, resolver] of this.pending) {
                resolver.reject(err)
            }
            this.pending.clear()
        }
    }

    call(method, args = [], timeoutMs = this.timeoutMs) {
        const id = ++this.seq
        return new Promise((resolve, reject) => {
            this.pending.set(id, { resolve, reject })
            this.worker.postMessage({ id, method, args })
            setTimeout(() => {
                if (!this.pending.has(id)) {
                    return
                }
                this.pending.delete(id)
                reject(new Error(`worker call timeout: ${method}`))
            }, timeoutMs)
        })
    }

    terminate() {
        this.worker.terminate()
    }
}

function buildProxy(facade, client) {
    return new Proxy(facade, {
        get(target, prop, receiver) {
            if (prop in target) {
                return Reflect.get(target, prop, receiver)
            }
            const value = client[prop]
            if (typeof value === 'function') {
                return value.bind(client)
            }
            return value
        },
        set(target, prop, value, receiver) {
            if (prop in target) {
                return Reflect.set(target, prop, value, receiver)
            }
            client[prop] = value
            return true
        },
    })
}

/**
 * Create a worker-backed client wrapper.
 *
 * The returned object is API-compatible with `new Client(info, dbName)` and forwards
 * all methods/properties to the underlying client except `syncChatLogs`, which is routed
 * to worker by default for better UI responsiveness.
 */
export async function createWorkerClient(info, dbName = '', options = {}) {
    const rpcTimeoutMs = options.rpcTimeoutMs ?? 8000
    const initTimeoutMs = options.initTimeoutMs ?? 5000
    const forceFallback = options.forceFallback ?? false

    const client = new Client(info, dbName)

    let workerEnabled = false
    let rpc = null

    if (!forceFallback) {
        try {
            const workerUrl = options.workerUrl ?? new URL('./entry.mjs', import.meta.url)
            const worker = new Worker(workerUrl, { type: 'module' })
            rpc = new WorkerRpc(worker, rpcTimeoutMs)
            await rpc.call('init', [info, dbName], initTimeoutMs)
            workerEnabled = true
        } catch (_e) {
            workerEnabled = false
            if (rpc) {
                rpc.terminate()
            }
            rpc = null
        }
    }

    const facade = {
        get workerEnabled() {
            return workerEnabled
        },

        async syncChatLogs(topicId, lastSeq, option = {}) {
            if (!workerEnabled || option.worker === false) {
                let result = null
                let failReason = null
                await client.syncChatLogs(topicId, lastSeq, {
                    ...option,
                    onsuccess: (r) => {
                        result = r
                        if (typeof option.onsuccess === 'function') {
                            option.onsuccess(r)
                        }
                    },
                    onfail: (e) => {
                        failReason = e
                        if (typeof option.onfail === 'function') {
                            option.onfail(e)
                        }
                    },
                })
                if (failReason && typeof option.onfail !== 'function') {
                    throw new Error(
                        typeof failReason === 'string' ? failReason : JSON.stringify(failReason)
                    )
                }
                return result
            }

            try {
                const result = await rpc.call('syncChatLogs', [topicId, lastSeq, option])
                if (typeof option.onsuccess === 'function') {
                    option.onsuccess(result)
                }
                return result
            } catch (e) {
                if (typeof option.onfail === 'function') {
                    option.onfail(e?.message || String(e))
                    return
                }
                throw e
            }
        },

        async shutdown() {
            try {
                if (workerEnabled && rpc) {
                    await rpc.call('shutdown', [])
                }
            } finally {
                if (rpc) {
                    rpc.terminate()
                    rpc = null
                    workerEnabled = false
                }
            }
            await client.shutdown()
        },
    }

    return buildProxy(facade, client)
}
