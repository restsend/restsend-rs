import WebSocket from 'ws'
import fs from 'node:fs'
import path from 'node:path'

/** HttpApi implementation on top of global fetch. */
export class NodeHttp {
  async request(method, url, headers, body) {
    const resp = await fetch(url, {
      method,
      headers,
      body: method === 'GET' || body === '' ? undefined : body
    })
    const text = await resp.text()
    return { status: resp.status, body: text }
  }
}

/** WsTransport implementation on top of the ws package (header-capable). */
export class NodeWs {
  connect(url, headers, events) {
    return new Promise((resolve, reject) => {
      const ws = new WebSocket(url, { headers })
      this.ws = ws
      const timer = setTimeout(() => {
        try { ws.terminate() } catch {}
        reject(new Error('handshake timeout'))
      }, 10000)
      ws.on('open', () => {
        clearTimeout(timer)
        events.onOpen()
        resolve(101)
      })
      ws.on('message', (data) => {
        events.onMessage(data.toString())
      })
      ws.on('close', (code) => {
        events.onClose(`code ${code}`)
      })
      ws.on('error', (err) => {
        events.onError(err.message)
      })
      // handshake rejected (e.g. 401): resolve the status, no open
      ws.on('unexpected-response', (req, res) => {
        clearTimeout(timer)
        resolve(res.statusCode)
      })
    })
  }

  send(text) {
    this.ws.send(text)
  }

  close() {
    try {
      this.ws.close()
    } catch {}
  }

  /** simulate a hard network drop */
  kill() {
    this.ws.terminate()
  }
}

/** MediaApi on top of native fetch + FormData (multipart field "file"). */
export class NodeMedia {
  async upload(req) {
    const data = fs.readFileSync(req.filePath)
    const form = new FormData()
    form.append('file', new Blob([data]), req.fileName)
    const resp = await fetch(req.url, { method: 'POST', headers: req.headers, body: form })
    if (req.onProgress) {
      req.onProgress(data.length, data.length)
    }
    return { status: resp.status, body: await resp.text() }
  }

  async uploadBytes(req) {
    const form = new FormData()
    form.append('file', new Blob([req.bytes]), req.fileName)
    const resp = await fetch(req.url, { method: 'POST', headers: req.headers, body: form })
    return { status: resp.status, body: await resp.text() }
  }

  async uploadContent(req) {
    const form = new FormData()
    form.append('file', new Blob([req.content]), req.fileName)
    const resp = await fetch(req.url, { method: 'POST', headers: req.headers, body: form })
    return { status: resp.status, body: await resp.text() }
  }

  async download(url, headers, savePath) {
    const resp = await fetch(url, { headers })
    if (resp.status !== 200) {
      throw new Error('download http ' + resp.status)
    }
    const buf = Buffer.from(await resp.arrayBuffer())
    fs.mkdirSync(path.dirname(savePath), { recursive: true })
    fs.writeFileSync(savePath, buf)
  }
}
