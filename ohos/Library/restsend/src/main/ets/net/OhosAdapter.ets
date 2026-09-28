import { http } from '@kit.NetworkKit'
import { webSocket } from '@kit.NetworkKit'
import fs from '@ohos.file.fs'
import { util } from '@kit.ArkTS'
import { BusinessError } from '@kit.BasicServicesKit'
import { HttpResponse, HttpApi, WsEvents, WsTransport } from './Transport'
import { UploadRequest, UploadContentRequest, MediaApi } from './MediaApi'

const TIMEOUT_MS = 60000

/** HttpApi implemented with @ohos.net.http. */
export class OhosHttp implements HttpApi {
  async request(method: string, url: string, headers: Record<string, string>, body: string): Promise<HttpResponse> {
    const req = http.createHttp()
    try {
      const options: http.HttpRequestOptions = {
        method: method === 'GET' ? http.RequestMethod.GET : http.RequestMethod.POST,
        header: headers,
        extraData: body === '' ? undefined : body,
        connectTimeout: TIMEOUT_MS,
        readTimeout: TIMEOUT_MS
      }
      const resp = await req.request(url, options)
      const text = typeof resp.result === 'string' ? resp.result : ''
      return { status: resp.responseCode, body: text }
    } finally {
      req.destroy()
    }
  }
}

/**
 * WsTransport implemented with @ohos.net.webSocket.
 * Note: ohos does not expose the handshake http status, so connect()
 * resolves 200 on success and 0 on failure; 401 detection falls back to
 * the generic error path (device-validation item).
 */
export class OhosWs implements WsTransport {
  private ws: webSocket.WebSocket | null = null

  connect(url: string, headers: Record<string, string>, events: WsEvents): Promise<number> {
    return new Promise<number>((resolve) => {
      const ws = webSocket.createWebSocket()
      this.ws = ws
      let opened = false
      ws.on('message', (err: BusinessError, value: string | ArrayBuffer) => {
        if (!err && typeof value === 'string') {
          events.onMessage(value)
        }
      })
      ws.on('close', (err: BusinessError, value: webSocket.CloseResult) => {
        events.onClose(`code ${value.code}`)
      })
      ws.on('error', (err: BusinessError) => {
        events.onError(err.message)
        if (!opened) {
          resolve(0)
        }
      })
      const options: webSocket.WebSocketRequestOptions = { header: headers }
      ws.connect(url, options).then((ok: boolean) => {
        opened = ok
        if (ok) {
          events.onOpen()
          resolve(200)
        } else {
          resolve(0)
        }
      }).catch((e: BusinessError) => {
        events.onError(e.message)
        resolve(0)
      })
    })
  }

  send(text: string): void {
    if (this.ws !== null) {
      this.ws.send(text)
    }
  }

  close(): void {
    if (this.ws !== null) {
      this.ws.close()
      this.ws = null
    }
  }
}

/**
 * MediaApi with hand-assembled multipart (field "file") for uploads and
 * ARRAY_BUFFER downloads. Upload progress is reported once at completion
 * (@ohos http has no upload progress events; switch to @ohos.request if
 * real progress is needed).
 */
export class OhosMedia implements MediaApi {
  async upload(req: UploadRequest): Promise<HttpResponse> {
    const stat: fs.Stat = fs.statSync(req.filePath)
    const file: fs.File = fs.openSync(req.filePath, fs.OpenMode.READ_ONLY)
    const fileBuf = new ArrayBuffer(stat.size)
    fs.readSync(file.fd, fileBuf)
    fs.closeSync(file.fd)
    const resp = await this.postMultipart(req.url, req.headers, req.fileName,
      new Uint8Array(fileBuf))
    req.onProgress?.(stat.size, stat.size)
    return resp
  }

  async uploadContent(req: UploadContentRequest): Promise<HttpResponse> {
    const encoder = new util.TextEncoder()
    return this.postMultipart(req.url, req.headers, req.fileName,
      encoder.encodeInto(req.content))
  }

  private async postMultipart(url: string, authHeaders: Record<string, string>,
    fileName: string, bytes: Uint8Array): Promise<HttpResponse> {
    const boundary = '----restsend' + String(Date.now())
    const pre = `--${boundary}\r\nContent-Disposition: form-data; name="file"; filename="${fileName}"\r\nContent-Type: application/octet-stream\r\n\r\n`
    const tail = `\r\n--${boundary}--\r\n`
    const encoder = new util.TextEncoder()
    const preBuf: Uint8Array = encoder.encodeInto(pre)
    const tailBuf: Uint8Array = encoder.encodeInto(tail)
    const total = preBuf.length + bytes.length + tailBuf.length
    const body = new Uint8Array(total)
    body.set(preBuf, 0)
    body.set(bytes, preBuf.length)
    body.set(tailBuf, preBuf.length + bytes.length)

    const headers: Record<string, string> = {}
    Object.keys(authHeaders).forEach((k) => {
      headers[k] = authHeaders[k]
    })
    headers['Content-Type'] = `multipart/form-data; boundary=${boundary}`

    const httpReq = http.createHttp()
    try {
      const options: http.HttpRequestOptions = {
        method: http.RequestMethod.POST,
        header: headers,
        extraData: body.buffer,
        connectTimeout: TIMEOUT_MS,
        readTimeout: TIMEOUT_MS
      }
      const resp = await httpReq.request(url, options)
      const text = typeof resp.result === 'string' ? resp.result : ''
      return { status: resp.responseCode, body: text }
    } finally {
      httpReq.destroy()
    }
  }

  async download(url: string, headers: Record<string, string>, savePath: string): Promise<void> {
    const httpReq = http.createHttp()
    try {
      const options: http.HttpRequestOptions = {
        method: http.RequestMethod.GET,
        header: headers,
        expectDataType: http.HttpDataType.ARRAY_BUFFER,
        connectTimeout: TIMEOUT_MS,
        readTimeout: TIMEOUT_MS
      }
      const resp = await httpReq.request(url, options)
      if (resp.responseCode !== 200) {
        throw new Error(`download http ${resp.responseCode}`)
      }
      const target = fs.openSync(savePath, fs.OpenMode.READ_WRITE | fs.OpenMode.CREATE | fs.OpenMode.TRUNC)
      fs.writeSync(target.fd, resp.result as ArrayBuffer)
      fs.closeSync(target.fd)
    } finally {
      httpReq.destroy()
    }
  }
}
