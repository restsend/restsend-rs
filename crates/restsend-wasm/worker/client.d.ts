export interface WorkerClientOptions {
    workerUrl?: string | URL
    rpcTimeoutMs?: number
    initTimeoutMs?: number
    forceFallback?: boolean
}

export declare function createWorkerClient(
    info: any,
    dbName?: string,
    options?: WorkerClientOptions
): Promise<any>
