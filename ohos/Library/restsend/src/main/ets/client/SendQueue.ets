import { ChatRequest } from '../model/ChatRequest'
import { nowMillis } from '../common/Ids'
import { SendCallback } from './Callback'

interface Pending {
  req: ChatRequest
  sentAt: number
  retries: number
  queuedAt: number
  cb?: SendCallback
}

/**
 * Tracks outbound chat requests: resp/ack matching, retry and offline
 * queueing. The connection layer injects the actual sender; typing/read
 * frames are fire-and-forget and handled by the client directly.
 */
export class SendQueue {
  private pending: Map<string, Pending> = new Map()
  private sender: (req: ChatRequest) => boolean = () => false
  private maxRetry: number
  private maxSendIdleSecs: number
  private maxPending: number
  private queuedLifetimeSecs: number

  constructor(maxRetry: number, maxSendIdleSecs: number,
    maxPending: number = 1000, queuedLifetimeSecs: number = 300) {
    this.maxRetry = maxRetry
    this.maxSendIdleSecs = maxSendIdleSecs
    this.maxPending = maxPending
    this.queuedLifetimeSecs = queuedLifetimeSecs
  }

  setSender(sender: (req: ChatRequest) => boolean): void {
    this.sender = sender
  }

  /** enqueue a chat request and try to send it right away */
  offer(req: ChatRequest, cb?: SendCallback): void {
    const chatId = req.chatId ?? ''
    if (chatId === '') {
      cb?.onFail?.('empty chatId')
      return
    }
    if (this.pending.size >= this.maxPending) {
      this.evictOldest('queue overflow')
    }
    this.pending.set(chatId, { req: req, sentAt: 0, retries: 0, queuedAt: nowMillis(), cb: cb })
    this.sendOne(chatId)
  }

  /** resend everything still unacked; called on (re)connect */
  flush(): void {
    this.pending.forEach((p, chatId) => this.sendOne(chatId))
  }

  private sendOne(chatId: string): void {
    const p = this.pending.get(chatId)
    if (p === undefined) {
      return
    }
    if (this.sender(p.req)) {
      p.sentAt = Date.now()
    }
  }

  /** resp frames from the server settle pending requests */
  handleAck(chatId: string, code: number, resp: ChatRequest): void {
    const p = this.pending.get(chatId)
    if (p === undefined) {
      return
    }
    this.pending.delete(chatId)
    if (code === 200) {
      p.cb?.onAck?.(resp)
    } else {
      p.cb?.onFail?.(resp.message ?? `code ${code}`)
    }
  }

  /** periodic timeout/retry sweep */
  checkTimeouts(): void {
    const now = Date.now()
    const expired: string[] = []
    this.pending.forEach((p, chatId) => {
      if (p.sentAt > 0 && now - p.sentAt > this.maxSendIdleSecs * 1000) {
        expired.push(chatId)
      }
    })
    expired.forEach((chatId) => {
      const p = this.pending.get(chatId)
      if (p === undefined) {
        return
      }
      if (p.retries < this.maxRetry) {
        p.retries += 1
        this.sendOne(chatId)
      } else {
        this.pending.delete(chatId)
        p.cb?.onFail?.('send timeout')
      }
    })
    // queued (never sent) entries must not accumulate forever either
    const queuedExpired: string[] = []
    this.pending.forEach((p, chatId) => {
      if (p.sentAt === 0 && nowMillis() - p.queuedAt > this.queuedLifetimeSecs * 1000) {
        queuedExpired.push(chatId)
      }
    })
    queuedExpired.forEach((chatId) => {
      const p = this.pending.get(chatId)
      if (p !== undefined) {
        this.pending.delete(chatId)
        p.cb?.onFail?.('queued too long')
      }
    })
  }

  failAll(reason: string): void {
    const ids: string[] = []
    this.pending.forEach((p, chatId) => ids.push(chatId))
    ids.forEach((chatId) => {
      const p = this.pending.get(chatId)
      if (p !== undefined) {
        this.pending.delete(chatId)
        p.cb?.onFail?.(reason)
      }
    })
  }

  /** drop a pending request without failing it (user cancel) */
  cancel(chatId: string): boolean {
    return this.pending.delete(chatId)
  }

  /** bound the offline queue: fail the oldest queued/sent entry */
  private evictOldest(reason: string): void {
    let oldestId = ''
    let oldestAt = Number.MAX_SAFE_INTEGER
    this.pending.forEach((p, chatId) => {
      const t = p.sentAt === 0 ? Number.MAX_SAFE_INTEGER - 1 : p.sentAt
      if (t < oldestAt) {
        oldestAt = t
        oldestId = chatId
      }
    })
    if (oldestId !== '') {
      const p = this.pending.get(oldestId)
      this.pending.delete(oldestId)
      p?.cb?.onFail?.(reason)
    }
  }

  size(): number {
    return this.pending.size
  }
}
