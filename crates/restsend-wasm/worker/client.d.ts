export interface WorkerClientOptions {
    /** Set to true to offload syncChatLogs to a Web Worker. Default: false */
    enableWorker?: boolean
    /** @deprecated Use enableWorker instead */
    forceFallback?: boolean
    /** Custom worker entry URL */
    workerUrl?: string | URL
    /** Worker RPC call timeout (ms). Default: 8000 */
    rpcTimeoutMs?: number
    /** Worker init timeout (ms). Default: 5000 */
    initTimeoutMs?: number
}

export declare function createWorkerClient(
    info: any,
    dbName?: string,
    options?: WorkerClientOptions
): Promise<any>
