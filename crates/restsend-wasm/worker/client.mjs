import { Client } from '../pkg/restsend_wasm.js';

class WorkerRpc {
    constructor(worker, timeoutMs) {
        this.worker = worker;
        this.timeoutMs = timeoutMs;
        this.seq = 0;
        this.pending = new Map();

        this.worker.onmessage = (event) => {
            const { id, ok, data, error } = event.data || {};
            const resolver = this.pending.get(id);
            if (!resolver) {
                return;
            }
            this.pending.delete(id);
            if (ok) {
                resolver.resolve(data);
            } else {
                resolver.reject(new Error(error || 'worker call failed'));
            }
        };

        this.worker.onerror = (event) => {
            const err = new Error(event?.message || 'worker runtime error');
            for (const [, resolver] of this.pending) {
                resolver.reject(err);
            }
            this.pending.clear();
        };
    }

    call(method, args = [], timeoutMs = this.timeoutMs) {
        const id = ++this.seq;
        return new Promise((resolve, reject) => {
            this.pending.set(id, { resolve, reject });
            this.worker.postMessage({ id, method, args });
            setTimeout(() => {
                if (!this.pending.has(id)) {
                    return;
                }
                this.pending.delete(id);
                reject(new Error(`worker call timeout: ${method}`));
            }, timeoutMs);
        });
    }

    terminate() {
        this.worker.terminate();
    }
}

function buildProxy(facade, client) {
    return new Proxy(facade, {
        get(target, prop, receiver) {
            if (prop in target) {
                return Reflect.get(target, prop, receiver);
            }
            const value = client[prop];
            if (typeof value === 'function') {
                return value.bind(client);
            }
            return value;
        },
        set(target, prop, value, receiver) {
            if (prop in target) {
                return Reflect.set(target, prop, value, receiver);
            }
            client[prop] = value;
            return true;
        },
    });
}

/**
 * Create a client wrapper with optional Web Worker support for syncChatLogs.
 *
 * By default the worker is **disabled**. Set `options.enableWorker = true` to offload
 * `syncChatLogs` calls to a Web Worker, keeping the main thread responsive during
 * heavy log fetching.
 *
 * @param {Object} info - Auth info (same as `new Client(info, dbName)`)
 * @param {string} [dbName=''] - IndexedDB database name
 * @param {Object} [options={}]
 * @param {boolean} [options.enableWorker=false] - Set to true to use a Web Worker for syncChatLogs
 * @param {number} [options.rpcTimeoutMs=8000] - Worker RPC call timeout (ms)
 * @param {number} [options.initTimeoutMs=5000] - Worker init timeout (ms)
 * @param {string} [options.workerUrl] - Custom worker entry URL
 * @returns {Promise<Object>} A proxy object that delegates to the underlying client
 */
export async function createWorkerClient(info, dbName = '', options = {}) {
    const rpcTimeoutMs = options.rpcTimeoutMs ?? 8000;
    const initTimeoutMs = options.initTimeoutMs ?? 5000;
    const enableWorker = options.enableWorker === true;

    const client = new Client(info, dbName);

    let workerEnabled = false;
    let rpc = null;

    if (enableWorker) {
        try {
            const workerUrl = options.workerUrl ?? new URL('./entry.mjs', import.meta.url);
            const worker = new Worker(workerUrl, { type: 'module' });
            rpc = new WorkerRpc(worker, rpcTimeoutMs);
            await rpc.call('init', [info, dbName], initTimeoutMs);
            workerEnabled = true;
        } catch (_e) {
            workerEnabled = false;
            if (rpc) {
                rpc.terminate();
            }
            rpc = null;
            console.error('[WorkerClient] Worker creation failed:', _e);
        }
    }

    const facade = {
        get workerEnabled() {
            return workerEnabled;
        },

        async syncChatLogs(topicId, lastSeq, option = {}) {
            console.log('[WorkerClient] syncChatLogs', workerEnabled, option.worker);
            if (!workerEnabled || option.worker === false) {
                let result = null;
                let failReason = null;
                await client.syncChatLogs(topicId, lastSeq, {
                    ...option,
                    onsuccess: (r) => {
                        result = r;
                        if (typeof option.onsuccess === 'function') {
                            option.onsuccess(r);
                        }
                    },
                    onfail: (e) => {
                        failReason = e;
                        if (typeof option.onfail === 'function') {
                            option.onfail(e);
                        }
                    },
                });
                if (failReason && typeof option.onfail !== 'function') {
                    throw new Error(typeof failReason === 'string' ? failReason : JSON.stringify(failReason));
                }
                return result;
            }

            try {
                const { onsuccess, onfail, ...serializableOption } = option;
                const result = await rpc.call('syncChatLogs', [topicId, lastSeq, serializableOption]);
                if (typeof option.onsuccess === 'function') {
                    option.onsuccess(result);
                }
                return result;
            } catch (e) {
                console.error('[WorkerClient] syncChatLogs failed:', e);
                if (typeof option.onfail === 'function') {
                    option.onfail(e?.message || String(e));
                    return;
                }
                throw e;
            }
        },

        async shutdown() {
            try {
                if (workerEnabled && rpc) {
                    await rpc.call('shutdown', []);
                }
            } finally {
                if (rpc) {
                    rpc.terminate();
                    rpc = null;
                    workerEnabled = false;
                }
            }
            await client.shutdown();
        },
    };

    return buildProxy(facade, client);
}
