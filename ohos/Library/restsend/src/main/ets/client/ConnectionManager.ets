import { ClientOptions } from './ClientOptions'
import { WsTransport, WsEvents, connectUrl } from '../net/Transport'
import { ReqType } from '../model/ChatRequest'
import { jnum } from '../common/Json'

export type ConnState = 'connecting' | 'connected' | 'broken' | 'shutdowned'

/** Internal events delivered by ConnectionManager. */
export class ConnEvents {
  onStatus: (s: ConnState) => void = (s: ConnState): void => {}
  onConnected: () => void = (): void => {}
  onBroken: (reason: string) => void = (reason: string): void => {}
  onConnectError: (reason: string) => void = (reason: string): void => {}
  onUnauthorized: () => void = (): void => {}
  onPingFailed: (reason: string) => void = (reason: string): void => {}
  onFrame: (text: string) => void = (text: string): void => {}
}

interface PingFrame {
  chatId: string
  text: string
}

interface PingContent {
  type: string
  text: string
}

interface PingWire {
  type: string
  chatId: string
  content: PingContent
}

function buildPingFrame(errorLogs: string[]): PingFrame {
  const chars = 'abcdefghijklmnopqrstuvwxyz0123456789'
  let chatId = ''
  for (let i = 0; i < 10; i++) {
    chatId += chars.charAt(Math.floor(Math.random() * chars.length))
  }
  const payload: Record<string, Object> = { 'timestamp': Date.now(), 'error_logs': errorLogs }
  const content: PingContent = { type: 'ping', text: JSON.stringify(payload) }
  const req: PingWire = { type: ReqType.Ping, chatId: chatId, content: content }
  const frame: PingFrame = { chatId: chatId, text: JSON.stringify(req) }
  return frame
}

/** Owned by RestsendClient: ws lifecycle, reconnect backoff and keepalive. */
export class ConnectionManager {
  status: ConnState = 'broken'
  lastAliveAt: number = 0

  private factory: () => WsTransport
  private endpoint: string
  private token: string
  private opts: ClientOptions
  private events: ConnEvents
  private ws: WsTransport | null = null
  private brokenCount = 0
  private unauthorizedCount = 0
  private wakeRequested = false
  private stopRequested = false
  private pingChatId = ''
  private pingSentAt = 0
  private errorLogs: string[] = []
  private pingTimer: number = -1
  /** mutable at runtime (rust setKeepaliveIntervalSecs) */
  private keepaliveSecs: number

  constructor(factory: () => WsTransport, endpoint: string, token: string,
    opts: ClientOptions, events: ConnEvents) {
    this.factory = factory
    this.endpoint = endpoint
    this.token = token
    this.opts = opts
    this.events = events
    this.keepaliveSecs = opts.pingIntervalSecs
  }

  /** rust setKeepaliveIntervalSecs */
  setKeepaliveIntervalSecs(secs: number): void {
    this.keepaliveSecs = secs > 0 ? secs : this.opts.pingIntervalSecs
    if (this.status === 'connected') {
      this.startPing()
    }
  }

  updateToken(token: string): void {
    this.token = token
  }

  addErrorLog(line: string): void {
    this.errorLogs.push(line)
    if (this.errorLogs.length > 100) {
      this.errorLogs.shift()
    }
  }

  /** main loop; resolves when shutdown() is called */
  async run(): Promise<void> {
    while (!this.stopRequested) {
      await this.connectOnce()
      if (this.stopRequested) {
        break
      }
      this.setStatus('broken')
      this.brokenCount += 1
      const delay = Math.min(this.brokenCount, this.opts.maxConnectIntervalSecs) * 1000
      await this.interruptibleSleep(delay)
    }
    this.setStatus('shutdowned')
  }

  /** foreground activation: retry immediately if we are waiting */
  appActive(): void {
    if (this.status === 'broken') {
      this.wakeRequested = true
    }
  }

  /** true after shutdown() or an unauthorized circuit break */
  isStopped(): boolean {
    return this.stopRequested
  }

  shutdown(): void {
    this.stopRequested = true
    this.wakeRequested = true
    if (this.ws !== null) {
      this.ws.close()
    }
  }

  send(text: string): boolean {
    if (this.status !== 'connected' || this.ws === null) {
      return false
    }
    try {
      this.ws.send(text)
      return true
    } catch (e) {
      this.addErrorLog(`ws send: ${e}`)
      this.ws.close()
      return false
    }
  }

  private setStatus(s: ConnState): void {
    if (this.status !== s) {
      this.status = s
      this.events.onStatus(s)
    }
  }

  private interruptibleSleep(ms: number): Promise<void> {
    return new Promise<void>((resolve) => {
      let waited = 0
      const step = 200
      const timer = setInterval(() => {
        waited += step
        if (this.wakeRequested || this.stopRequested || waited >= ms) {
          this.wakeRequested = false
          clearInterval(timer)
          resolve()
        }
      }, step)
    })
  }

  private connectOnce(): Promise<void> {
    return new Promise<void>((resolve) => {
      let opened = false
      let done = false
      const finish = (): void => {
        if (!done) {
          done = true
          this.stopPing()
          resolve()
        }
      }
      // hard hang guard: webSocket.connect has no built-in timeout
      const handshakeTimer = setTimeout((): void => {
        if (!done && !opened) {
          this.events.onConnectError('handshake timeout')
          try {
            this.ws?.close()
          } catch (e) {
            this.addErrorLog(`ws close: ${e}`)
          }
          finish()
        }
      }, 15000)
      const events = new WsEvents()
      events.onOpen = (): void => {
        opened = true
        this.brokenCount = 0
        this.unauthorizedCount = 0
        clearTimeout(handshakeTimer)
        this.lastAliveAt = Date.now()
        this.setStatus('connected')
        this.events.onConnected()
        this.startPing()
      }
      events.onMessage = (text: string): void => {
        this.lastAliveAt = Date.now()
        this.events.onFrame(text)
      }
      events.onClose = (reason: string): void => {
        clearTimeout(handshakeTimer)
        if (opened) {
          this.events.onBroken(reason)
        }
        finish()
      }
      events.onError = (reason: string): void => {
        this.addErrorLog(`ws error: ${reason}`)
        if (!opened) {
          clearTimeout(handshakeTimer)
          this.events.onConnectError(reason)
          finish()
        }
      }
      const ws = this.factory()
      this.ws = ws
      const nonce = this.opts.nonce !== '' ? this.opts.nonce : Math.random().toString(36).substring(2, 6)
      const headers: Record<string, string> = {
        'Authorization': `Bearer ${this.token}`,
        'User-Agent': `restsend-arkts/${this.opts.version}`
      }
      ws.connect(connectUrl(this.endpoint, this.opts.version, nonce), headers, events).then((status: number) => {
        if (status === 401) {
          // circuit breaker: 3 consecutive unauthorized -> stop retrying until
          // the host updates the token and reconnects
          this.unauthorizedCount += 1
          this.events.onUnauthorized()
          if (this.unauthorizedCount >= 3) {
            this.stopRequested = true
          }
          finish()
        } else if (status !== 0 && status !== 101 && status !== 200 && !opened) {
          this.events.onConnectError(`handshake status ${status}`)
          finish()
        }
      }).catch((e: Object) => {
        clearTimeout(handshakeTimer)
        this.addErrorLog(`ws connect: ${e}`)
        if (!opened) {
          finish()
        }
      })
    })
  }

  // ---- keepalive ----

  private startPing(): void {
    this.stopPing()
    const interval = this.keepaliveSecs * 1000
    this.pingTimer = setInterval(() => this.sendPing(), interval)
  }

  private stopPing(): void {
    if (this.pingTimer >= 0) {
      clearInterval(this.pingTimer)
      this.pingTimer = -1
    }
  }

  private sendPing(): void {
    if (this.status !== 'connected') {
      return
    }
    const frame = buildPingFrame(this.errorLogs)
    this.pingChatId = frame.chatId
    this.pingSentAt = Date.now()
    if (!this.send(frame.text)) {
      this.events.onPingFailed('send failed')
      if (this.ws !== null) {
        this.ws.close()
      }
    }
  }

  /** returns true when the resp settles our pending ping */
  handleResp(chatId: string, contentText: string | undefined): boolean {
    if (this.pingChatId === '' || chatId !== this.pingChatId) {
      return false
    }
    this.pingChatId = ''
    if (contentText !== undefined) {
      try {
        const payload = JSON.parse(contentText) as Record<string, Object>
        const serverTime = jnum(payload, 'timestamp')
        if (serverTime > 0 && Date.now() - serverTime > this.opts.pingTimeoutSecs * 1000) {
          this.events.onPingFailed(`clock diff ${Date.now() - serverTime}ms`)
        }
      } catch (e) {
        // ignore malformed ping payload
      }
    }
    return true
  }

  pingTimeout(): boolean {
    return this.pingChatId !== '' && Date.now() - this.pingSentAt > this.opts.pingTimeoutSecs * 1000
  }
}
