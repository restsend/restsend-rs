const { Client, signin } = await import('../pkg/restsend_wasm.js')

export const endpoint = 'https://chat.ruzhila.cn/'
export async function waitUntil(fn, timeout) {
    let start = Date.now()
    while (true) {
        if (fn()) {
            return true
        }
        if (Date.now() - start > timeout) {
            return false
        }
        await new Promise(resolve => setTimeout(resolve, 100))
    }
}

async function clearDatabase(name) {
    await new Promise((resolve, reject) => {
        let req = window.indexedDB.deleteDatabase(name)
        req.onsuccess = resolve
        req.onerror = reject
    })
}
export async function authClient(username, password, withWebSocket = false) {
    if (typeof window.indexedDB !== 'undefined') {
        let tbls = ["topics", "users", "messages", "conversations", "chat_logs"]
        for (let tbl of tbls) {
            await clearDatabase(`${username}-${tbl}`)
            console.log(`clear ${username}-${tbl}`)
        }
    }
    let info = await signin(endpoint, username, password)
    let client = new Client(info)

    if (withWebSocket) {
        await client.connect()
        await waitUntil(() => client.connectionStatus === 'connected', 3000)
    }
    return client
}
