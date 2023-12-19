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

export async function authClient(username, password, withWebSocket = false) {
    let info = await signin(endpoint, username, password)
    let client = new Client(endpoint, username, info.token)

    if (withWebSocket) {
        await client.connect()
        await waitUntil(() => client.connection_status === 'connected', 3000)
    }
    return client
}
