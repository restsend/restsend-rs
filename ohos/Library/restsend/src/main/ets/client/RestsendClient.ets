import { AuthInfo, Extra, User, parseUser } from '../model/User'
import { Conversation, Tag } from '../model/Conversation'
import { ChatLog, ChatLogStatus, chatLogFromRequest } from '../model/ChatLog'
import { ChatRequest, ReqType, parseChatRequestText } from '../model/ChatRequest'
import { Content, ContentType, AttachmentStatus, Attachment } from '../model/Content'
import { Topic, TopicMembersResult, parseTopic } from '../model/Topic'
import { TopicKnock, parseTopicKnocks, GetChatLogsResult, ApiSendResponse, parseApiSendResponse } from '../model/TopicKnock'
import { Upload } from '../model/Upload'
import { WsTransport } from '../net/Transport'
import { UploadRequest, UploadContentRequest, MediaApi } from '../net/MediaApi'
import { Table } from '../store/Table'
import { Api, TopicCreateOptions } from '../api/Api'
import { Json, jnum, parseJson } from '../common/Json'
import { ClientError, ClientErrorKind } from '../common/ClientError'
import { randomChatId, nowMillis } from '../common/Ids'
import { newUpdateExtraRequest, newTextRequest, newLocationRequest, newLinkRequest,
  newInviteRequest, newLogsRequest, doApplySendOptions, SendOptions } from '../protocol/Requests'
import { AttachmentManager } from './AttachmentManager'
import { ClientOptions } from './ClientOptions'
import { ConnectionManager, ConnEvents, ConnState } from './ConnectionManager'
import { SendQueue } from './SendQueue'
import { SendCallback, DownloadCallback, MessageStatus, SyncResult, SyncChatLogsResult } from './Callback'
import { ClientEvents } from './ClientEvents'
import { ConversationManager, isCountableContent } from './ConversationManager'

/**
 * RestSend client facade, ported from the Rust core (crates/restsend).
 * Transports are injected: pass a @ohos based WsTransport factory on device,
 * or a node based one in tests.
 *
 * Usage:
 *   const client = new RestsendClient(info, wsFactory)
 *   client.events.onConnected = (): void => { ... }
 *   await client.connect()
 */
export class RestsendClient {
  readonly info: AuthInfo
  readonly api: Api
  readonly opts: ClientOptions
  readonly events: ClientEvents = new ClientEvents()
  readonly attachments: AttachmentManager | null

  private conn: ConnectionManager
  private wsFactory: () => WsTransport
  private queue: SendQueue
  private manager: ConversationManager
  private sweeper: number = -1
  private lastUpdatedAt = ''
  private lastRemovedAt = ''
  private gapFilling: Set<string> = new Set()
  private lastFillAt: Map<string, number> = new Map()
  private lastMaintenance: number = nowMillis()
  private userStore: Table | null = null
  private userCache: Map<string, User> = new Map()
  private userCachedAt: Map<string, number> = new Map()
  private countable: ((content: Content) => boolean) | null = null
  private logTopicByChatId: Map<string, string> = new Map()
  private closed = false

  constructor(info: AuthInfo, wsFactory: () => WsTransport, store?: Table,
    opts?: ClientOptions, media?: MediaApi) {
    this.info = info
    this.opts = opts ?? new ClientOptions()
    this.api = new Api(info.endpoint, info.token)
    this.attachments = media !== undefined ? new AttachmentManager(media, this.api) : null
    this.userStore = store ?? null
    this.queue = new SendQueue(this.opts.maxRetry, this.opts.maxSendIdleSecs,
      this.opts.maxPendingSends, this.opts.queuedLifetimeSecs)
    this.manager = new ConversationManager(store ?? null, this.opts.maxLogsLimit)

    const connEvents = new ConnEvents()
    connEvents.onStatus = (s: ConnState): void => {
      if (s === 'connecting') {
        this.fire((): void => this.events.onConnecting())
      }
    }
    connEvents.onConnected = (): void => {
      this.queue.flush()
      this.fire((): void => this.events.onConnected())
    }
    connEvents.onBroken = (reason: string): void => {
      this.fire((): void => this.events.onBroken(reason))
    }
    connEvents.onConnectError = (reason: string): void => {
      this.conn.addErrorLog(reason)
    }
    connEvents.onUnauthorized = (): void => {
      this.fire((): void => this.events.onTokenExpired('unauthorized'))
    }
    connEvents.onPingFailed = (reason: string): void => {
      this.fire((): void => this.events.onPingFailed(reason))
    }
    connEvents.onFrame = (text: string): void => {
      try {
        this.handleFrame(text)
      } catch (e) {
        this.conn.addErrorLog(`frame error: ${e}`)
      }
    }
    this.wsFactory = wsFactory
    this.conn = new ConnectionManager(wsFactory, info.endpoint, info.token, this.opts, connEvents)
    this.queue.setSender((req: ChatRequest): boolean => this.conn.send(JSON.stringify(req)))
  }

  /** host callbacks must never crash the sdk loop */
  private fire(f: () => void): void {
    try {
      f()
    } catch (e) {
      this.conn.addErrorLog(`event error: ${e}`)
    }
  }

  private fireStatus(f: () => MessageStatus): MessageStatus {
    try {
      return f()
    } catch (e) {
      this.conn.addErrorLog(`event error: ${e}`)
      const s: MessageStatus = { hasRead: false, unreadCountable: true }
      return s
    }
  }

  // ---- lifecycle ----

  async connect(): Promise<void> {
    if (this.closed) {
      return
    }
    await this.manager.hydrate()
    this.conn.run().catch((): void => {})
    this.sweeper = setInterval((): void => {
      try {
        this.sweepOnce()
      } catch (e) {
        this.conn.addErrorLog(`sweep error: ${e}`)
      }
    }, 1000)
    await this.syncConversations()
  }

  private sweepOnce(): void {
    this.queue.checkTimeouts()
    if (this.conn.pingTimeout()) {
      this.fire((): void => this.events.onPingFailed('ping timeout'))
    }
    if (nowMillis() - this.lastMaintenance > 3600000) {
      this.lastMaintenance = nowMillis()
      this.manager.maintain(this.opts.maxConversationLimit, this.opts.messageRetentionDays, this.opts.removedCacheTtlSecs)
    }
    // backstop gap scan: tail behind conversation, or hole in the middle
    this.conversations().forEach((c) => {
      if (this.manager.hasSeqHole(c.topicId) || c.lastSeq > this.manager.topSeq(c.topicId)) {
        this.fillGap(c.topicId, c.lastSeq)
      }
    })
  }

  shutdown(): void {
    this.closed = true
    if (this.sweeper >= 0) {
      clearInterval(this.sweeper)
      this.sweeper = -1
    }
    this.queue.failAll('shutdown')
    this.conn.shutdown()
  }

  appActive(): void {
    this.conn.appActive()
  }

  /** refresh credentials and resume connecting (after a 401 circuit break) */
  updateToken(token: string): void {
    this.api.token = token
    this.conn.updateToken(token)
    if (this.closed || this.conn.isStopped() || this.conn.status === 'shutdowned') {
      this.closed = false
      this.conn = this.rebuildConnection()
    }
    this.conn.appActive()
  }

  private rebuildConnection(): ConnectionManager {
    const connEvents = new ConnEvents()
    connEvents.onStatus = (s: ConnState): void => {
      if (s === 'connecting') {
        this.events.onConnecting()
      }
    }
    connEvents.onConnected = (): void => {
      this.queue.flush()
      this.events.onConnected()
    }
    connEvents.onBroken = (reason: string): void => {
      this.events.onBroken(reason)
    }
    connEvents.onConnectError = (reason: string): void => {
      this.conn.addErrorLog(reason)
    }
    connEvents.onUnauthorized = (): void => {
      this.events.onTokenExpired('unauthorized')
    }
    connEvents.onPingFailed = (reason: string): void => {
      this.events.onPingFailed(reason)
    }
    connEvents.onFrame = (text: string): void => {
      try {
        this.handleFrame(text)
      } catch (e) {
        this.conn.addErrorLog(`frame error: ${e}`)
      }
    }
    const conn = new ConnectionManager(this.wsFactory, this.info.endpoint, this.api.token,
      this.opts, connEvents)
    conn.run().catch((): void => {})
    return conn
  }

  /** load conversations/logs from local storage without touching the network */
  async hydrate(): Promise<void> {
    await this.manager.hydrate()
  }

  connectionStatus(): string {
    return this.conn.status
  }

  // ---- read model ----

  conversations(): Conversation[] {
    return this.manager.all()
  }

  conversation(topicId: string): Conversation | undefined {
    return this.manager.get(topicId)
  }

  getLogs(topicId: string, limit: number): ChatLog[] {
    return this.manager.getLogs(topicId, limit)
  }

  unreadCount(): number {
    return this.manager.totalUnread()
  }

  // ---- sending ----

  /** send a text message; returns the chatId (ack arrives via onAck) */
  doSendText(topicId: string, text: string, cb?: SendCallback, opts?: SendOptions): string {
    if (this.closed) {
      return ''
    }
    const req = newTextRequest(topicId, text, opts)
    this.sendChat(topicId, req, cb)
    return req.chatId ?? ''
  }

  /** send any pre-built content (rust doSend) */
  doSend(topicId: string, content: Content, cb?: SendCallback, opts?: SendOptions): string {
    if (this.closed) {
      return ''
    }
    const req: ChatRequest = {
      type: ReqType.Chat,
      chatId: randomChatId(),
      topicId: topicId,
      content: content
    }
    doApplySendOptions(req, opts)
    this.sendChat(topicId, req, cb)
    return req.chatId ?? ''
  }

  doRecall(topicId: string, chatId: string): string {
    const req: ChatRequest = {
      type: ReqType.Chat,
      chatId: randomChatId(),
      topicId: topicId,
      content: { type: ContentType.Recall, text: chatId } as Content
    }
    this.sendChat(topicId, req, undefined)
    const log = this.manager.findLog(topicId, chatId)
    if (log !== undefined) {
      log.recall = true
      this.fire((): void => this.events.onLogsChanged(topicId))
    }
    return req.chatId ?? ''
  }

  doTyping(topicId: string): void {
    const req: ChatRequest = { type: ReqType.Typing, topicId: topicId }
    this.conn.send(JSON.stringify(req))
  }

  /** enqueue a chat request and mirror its lifecycle into the local log */
  private sendChat(topicId: string, req: ChatRequest, cb?: SendCallback): void {
    const log = chatLogFromRequest(req)
    log.status = ChatLogStatus.Sending
    log.cachedAt = nowMillis()
    this.manager.trackOutgoing(log)
    this.logTopicByChatId.set(req.chatId ?? '', topicId)
    this.offerWithLog(topicId, req, cb)
    this.fire((): void => this.events.onLogsChanged(topicId))
  }

  /** queue.offer + ack/fail mirrored into the already-tracked local log */
  private offerWithLog(topicId: string, req: ChatRequest, cb?: SendCallback): void {
    this.logTopicByChatId.set(req.chatId ?? '', topicId)
    const wrapped = new SendCallback()
    wrapped.onAck = (r: ChatRequest): void => {
      this.logTopicByChatId.delete(req.chatId ?? '')
      const l = this.manager.findLog(topicId, req.chatId ?? '')
      if (l !== undefined) {
        l.status = ChatLogStatus.Sent
        l.seq = r.seq ?? l.seq
        const cur = this.manager.get(topicId)
        if (cur !== undefined && l.seq > cur.lastSeq) {
          cur.lastSeq = l.seq
          cur.lastMessage = l.content
          cur.lastMessageSeq = l.seq
          cur.lastSenderId = this.info.userId
        }
      }
      this.fire((): void => this.events.onLogsChanged(topicId))
      if (cb !== undefined) {
        cb.onAck(r)
      }
    }
    wrapped.onFail = (reason: string): void => {
      this.logTopicByChatId.delete(req.chatId ?? '')
      const l = this.manager.findLog(topicId, req.chatId ?? '')
      if (l !== undefined) {
        l.status = ChatLogStatus.SendFailed
      }
      this.fire((): void => this.events.onLogsChanged(topicId))
      if (cb !== undefined) {
        cb.onFail(reason)
      }
    }
    this.queue.offer(req, wrapped)
  }

  /**
   * Send an attachment message: upload first, then send the frame with the
   * resulting url/size. The local log shows sending until the ack arrives.
   */
  doSendAttachment(topicId: string, type: ContentType, filePath: string, fileName: string,
    cb?: SendCallback, durationSecs: number = 0): string {
    const chatId = randomChatId()
    if (this.attachments === null) {
      cb?.onFail('media adapter not configured')
      return chatId
    }
    const content: Content = {
      type: type,
      attachment: { url: '', size: 0, thumbnail: '', fileName: fileName, status: AttachmentStatus.Uploading }
    }
    if (durationSecs > 0) {
      content.duration = formatDuration(durationSecs)
    }
    const req: ChatRequest = { type: ReqType.Chat, chatId: chatId, topicId: topicId, content: content }
    const log = chatLogFromRequest(req)
    log.status = ChatLogStatus.Sending
    log.cachedAt = nowMillis()
    this.manager.trackOutgoing(log)
    this.logTopicByChatId.set(chatId, topicId)
    this.fire((): void => this.events.onLogsChanged(topicId))
    const onFailLocal = (reason: string): void => {
      const l = this.manager.findLog(topicId, chatId)
      if (l !== undefined) {
        l.status = ChatLogStatus.SendFailed
      }
      this.fire((): void => this.events.onLogsChanged(topicId))
      cb?.onFail(reason)
    }
    this.attachments.upload(filePath, fileName, (sent: number, total: number): void => {
      cb?.onProgress(sent, total)
    }).then((up: Upload): void => {
      content.attachment = {
        url: up.path, size: up.size, thumbnail: up.thumbnail,
        fileName: up.fileName !== '' ? up.fileName : fileName, ext: up.ext, status: AttachmentStatus.Done
      }
      if (up.size > 0) {
        content.size = up.size
      }
      this.offerWithLog(topicId, req, cb)
    }).catch((e: Object): void => {
      onFailLocal(String(e))
    })
    return chatId
  }

  doSendImage(topicId: string, filePath: string, fileName: string, cb?: SendCallback): string {
    return this.doSendAttachment(topicId, ContentType.Image, filePath, fileName, cb)
  }

  doSendVideo(topicId: string, filePath: string, fileName: string, cb?: SendCallback): string {
    return this.doSendAttachment(topicId, ContentType.Video, filePath, fileName, cb)
  }

  doSendVoice(topicId: string, filePath: string, fileName: string, durationSecs: number,
    cb?: SendCallback): string {
    return this.doSendAttachment(topicId, ContentType.Voice, filePath, fileName, cb, durationSecs)
  }

  doSendFile(topicId: string, filePath: string, fileName: string, cb?: SendCallback): string {
    return this.doSendAttachment(topicId, ContentType.File, filePath, fileName, cb)
  }

  doSendLocation(topicId: string, lat: string, lng: string, address: string,
    cb?: SendCallback, opts?: SendOptions): string {
    const req = newLocationRequest(topicId, lat, lng, address)
    doApplySendOptions(req, opts)
    this.sendChat(topicId, req, cb)
    return req.chatId ?? ''
  }

  doSendLink(topicId: string, url: string, placeholder: string,
    cb?: SendCallback, opts?: SendOptions): string {
    const req = newLinkRequest(topicId, url, placeholder)
    doApplySendOptions(req, opts)
    this.sendChat(topicId, req, cb)
    return req.chatId ?? ''
  }

  doSendInvite(topicId: string, message: string, cb?: SendCallback): string {
    const req = newInviteRequest(topicId, message)
    this.sendChat(topicId, req, cb)
    return req.chatId ?? ''
  }

  /** forward selected messages of sourceTopicId as a logs attachment */
  doSendLogs(topicId: string, sourceTopicId: string, logIds: string[], cb?: SendCallback): string {
    if (this.attachments === null) {
      cb?.onFail('media adapter not configured')
      return ''
    }
    const items: string[] = []
    for (const id of logIds) {
      const log = this.manager.findLog(sourceTopicId, id)
      if (log !== undefined) {
        items.push(JSON.stringify(log))
      }
    }
    const data = JSON.stringify({
      'topicId': sourceTopicId,
      'ownerId': this.info.userId,
      'createdAt': new Date(nowMillis()).toISOString(),
      'logIds': logIds,
      'items': items
    })
    const chatId = randomChatId()
    this.attachments.uploadContent('Chat history.json', data).then((up: Upload): void => {
      const attachment: Attachment = {
        url: up.path, size: up.size, thumbnail: up.thumbnail,
        fileName: up.fileName !== '' ? up.fileName : 'Chat history', ext: up.ext
      }
      const req = newLogsRequest(topicId, attachment)
      this.logTopicByChatId.set(req.chatId ?? '', topicId)
      this.offerWithLog(topicId, req, cb)
      this.fire((): void => this.events.onLogsChanged(topicId))
    }).catch((e: Object): void => {
      cb?.onFail(String(e))
    })
    return chatId
  }

  /** download a (possibly private) attachment to savePath */
  async downloadAttachment(url: string, savePath: string): Promise<void> {
    if (this.attachments === null) {
      throw new Error('media adapter not configured')
    }
    return this.attachments.download(url, savePath)
  }

  resolveUrl(url: string): string {
    if (this.attachments === null) {
      return url
    }
    return this.attachments.resolveUrl(url)
  }

  // ---- incoming frames ----

  private handleFrame(text: string): void {
    let req: ChatRequest | null = null
    try {
      req = parseChatRequestText(text)
    } catch (e) {
      req = null
    }
    if (req === null) {
      return
    }
    const frame = req
    switch (frame.type) {
      case ReqType.Resp:
        if (!this.conn.handleResp(frame.chatId ?? '', frame.content?.text)) {
          this.queue.handleAck(frame.chatId ?? '', frame.code ?? 0, frame)
        }
        return
      case ReqType.Chat:
        this.handleChat(frame)
        return
      case ReqType.Read:
        this.handleRead(this.normalizeTopic(frame), frame)
        return
      case ReqType.Typing:
        this.fire((): void => this.events.onTopicTyping(this.normalizeTopic(frame), frame.content?.text ?? ''))
        return
      case ReqType.Kickout:
      case ReqType.Kick:
        this.fire((): void => this.events.onKickoff(frame.message ?? 'kickout'))
        this.shutdown()
        return
      case ReqType.System:
        this.fire((): void => this.events.onSystemRequest(frame))
        return
      default:
        this.fire((): void => this.events.onUnknownRequest(frame))
        return
    }
  }

  /**
   * 1:1 conversations have per-side topic aliases ("me:peer"). The server
   * rewrites chat frames per recipient but passes typing/read through as
   * sent, so normalize them to our own alias.
   */
  private normalizeTopic(req: ChatRequest): string {
    const topicId = req.topicId ?? ''
    const idx = topicId.indexOf(':')
    if (idx > 0 && topicId.substring(0, idx) !== this.info.userId) {
      return topicId.substring(idx + 1) + ':' + topicId.substring(0, idx)
    }
    return topicId
  }

  private handleChat(req: ChatRequest): void {
    const topicId = req.topicId ?? ''
    if (topicId === '') {
      return
    }
    const contentType = req.content?.type ?? ''
    if (contentType === 'conversation.update') {
      if (this.manager.applyServerSnapshot(req)) {
        this.emitConversations()
      }
      return
    }
    if (contentType === 'conversation.removed') {
      this.manager.markRemoved(topicId, req.seq ?? 0)
      this.fire((): void => this.events.onConversationRemoved(topicId))
      return
    }
    if (contentType === ContentType.Recall || req.type === ReqType.Recall) {
      // recall frame: mark the referenced log instead of storing a new one
      const targetId = req.content?.text ?? ''
      const target = this.manager.findLog(topicId, targetId)
      if (target !== undefined) {
        target.recall = true
        this.fire((): void => this.events.onLogsChanged(topicId))
      }
      return
    }
    const log = this.manager.saveIncoming(req, this.info.userId)
    let hasRead = false
    let countable = isCountableContent(contentType)
    const status = this.fireStatus((): MessageStatus => this.events.onTopicMessage(topicId, req))
    if (status !== undefined && status !== null) {
      hasRead = status.hasRead
      countable = status.unreadCountable
    }
    if (req.content !== undefined && this.countable !== null) {
      countable = this.countable(req.content)
    }
    this.updateConversationFromChat(req, !hasRead && countable)
    if (hasRead) {
      this.setConversationRead(topicId, req.seq ?? 0)
    }
    if (log !== null) {
      this.fire((): void => this.events.onLogsChanged(topicId))
    }
  }

  private updateConversationFromChat(req: ChatRequest, bumpUnread: boolean): void {
    const topicId = req.topicId ?? ''
    const seq = req.seq ?? 0
    const cur = this.manager.get(topicId)
    if (cur === undefined) {
      // unknown topic: fetch its snapshot in the background
      this.syncConversations().catch((): void => {})
      return
    }
    const prevLastSeq = cur.lastSeq
    if (seq <= cur.lastSeq && (req.attendee ?? '') !== this.info.userId) {
      return // stale or already applied via conversation.update
    }
    cur.lastSeq = Math.max(cur.lastSeq, seq)
    cur.lastSenderId = req.attendee ?? ''
    cur.lastMessage = req.content
    cur.lastMessageSeq = seq
    cur.updatedAt = req.createdAt ?? cur.updatedAt
    if (bumpUnread) {
      cur.unread += 1
    }
    if (seq > prevLastSeq + 1 && prevLastSeq > 0) {
      this.fillGap(topicId, seq)
    }
    this.emitConversations()
  }

  /**
   * seq hole / tail lag detected: pull a window from the server and upsert.
   * /chat/sync pages BACKWARD (items <= lastSeq, newest first), so we start
   * at the newest known point and page older until the hole is covered.
   */
  private fillGap(topicId: string, target: number): void {
    if (this.gapFilling.has(topicId)) {
      return
    }
    const last = this.lastFillAt.get(topicId) ?? 0
    if (nowMillis() - last < 250) {
      return
    }
    this.lastFillAt.set(topicId, nowMillis())
    this.gapFilling.add(topicId)
    const run = async (): Promise<void> => {
      let cursor = 0
      for (let depth = 0; depth < 5; depth++) {
        const r = await this.syncChatLogs(topicId, cursor)
        if (!r.hasMore) {
          break
        }
        const min = this.manager.minSeq(topicId)
        if (min <= 0 || min <= cursor) {
          break
        }
        cursor = min
      }
    }
    run().catch((): void => {}).finally((): void => {
      this.gapFilling.delete(topicId)
    })
  }

  private handleRead(topicId: string, req: ChatRequest): void {
    const seq = req.seq ?? 0
    // attendee may be "userId:device" on read receipts
    const attendeeId = (req.attendee ?? '').split(':')[0]
    if (attendeeId === this.info.userId) {
      // another device of mine read up to seq
      this.manager.markLocallyRead(topicId, seq)
    }
    this.fire((): void => this.events.onTopicRead(topicId, req))
    this.emitConversations()
  }

  private emitConversations(): void {
    this.fire((): void => this.events.onConversationsUpdated(this.manager.all(), this.manager.totalUnread()))
  }

  // ---- read / sync ----

  /** open (or fetch) a 1:1 conversation with a user, merged locally */
  async createChat(userId: string): Promise<Conversation> {
    const conv = await this.api.chatCreate(userId)
    this.manager.merge(conv)
    this.emitConversations()
    return this.conversation(conv.topicId) ?? conv
  }

  /** remove a conversation: server delete + local removal with anti-refill tombstone */
  async removeConversation(topicId: string): Promise<void> {
    const cur = this.manager.get(topicId)
    const seq = cur?.lastSeq ?? 0
    await this.api.chatRemove(topicId)
    this.manager.markRemoved(topicId, seq)
    this.fire((): void => this.events.onConversationRemoved(topicId))
    this.emitConversations()
  }

  async syncConversations(limit: number = 50): Promise<SyncResult> {
    if (this.closed) {
      const empty: SyncResult = { updatedAt: '', count: 0, total: 0 }
      return empty
    }
    const r = await this.api.chatList(limit, this.lastUpdatedAt, this.lastRemovedAt)
    for (const removed of r.removed) {
      this.manager.markRemoved(removed.topicId, removed.seq)
      this.fire((): void => this.events.onConversationRemoved(removed.topicId))
    }
    for (const c of r.items) {
      this.manager.merge(c)
    }
    if (r.lastUpdatedAt !== '') {
      this.lastUpdatedAt = r.lastUpdatedAt
    }
    if (r.lastRemovedAt !== '') {
      this.lastRemovedAt = r.lastRemovedAt
    }
    this.emitConversations()
    const result: SyncResult = { updatedAt: r.lastUpdatedAt, count: r.items.length, total: r.total }
    return result
  }

  async syncChatLogs(topicId: string, lastSeq?: number, limit?: number): Promise<SyncChatLogsResult> {
    const cur = this.manager.get(topicId)
    const from = lastSeq ?? 0
    const fetchLimit = limit ?? this.opts.maxSyncLogsLimit
    const r = await this.api.chatSync(topicId, from, fetchLimit)
    let saved = 0
    for (const req of r.items) {
      const log = this.manager.saveIncoming(req, this.info.userId)
      if (log !== null) {
        saved += 1
      }
      if (cur !== undefined && (req.seq ?? 0) > cur.lastSeq) {
        cur.lastSeq = req.seq ?? 0
      }
    }
    const result: SyncChatLogsResult = { topicId: topicId, hasMore: r.hasMore, lastSeq: r.lastSeq, count: saved }
    if (saved > 0) {
      this.fire((): void => this.events.onLogsChanged(topicId))
    }
    return result
  }

  // ---- conversation settings ----

  async setConversationSticky(topicId: string, sticky: boolean): Promise<Conversation | undefined> {
    return this.updateConversationBy(topicId, { 'sticky': sticky })
  }

  async setConversationMute(topicId: string, mute: boolean): Promise<Conversation | undefined> {
    return this.updateConversationBy(topicId, { 'mute': mute })
  }

  async setConversationRemark(topicId: string, remark: string): Promise<Conversation | undefined> {
    return this.updateConversationBy(topicId, { 'remark': remark })
  }

  /** conversation extra merged per key, via REST */
  async setConversationExtra(topicId: string, key: string, value: Object): Promise<void> {
    const extra: Record<string, Object> = {}
    extra[key] = value
    await this.updateConversationBy(topicId, { 'extra': extra })
  }

  /** update one chat log's extra via ws frame (rust doUpdateExtra) */
  doUpdateExtra(topicId: string, targetChatId: string, extra: Extra): string {
    const req: ChatRequest = newUpdateExtraRequest(topicId, targetChatId, extra)
    this.queue.offer(req)
    return req.chatId ?? ''
  }

  private async updateConversationBy(topicId: string, fields: Record<string, Object>): Promise<Conversation | undefined> {
    await this.api.chatUpdate(topicId, fields)
    this.manager.updateFields(topicId, fields)
    this.emitConversations()
    return this.conversation(topicId)
  }

  // ---- topics (groups) ----

  createTopic(name: string, memberIds: string[], icon: string = '',
    opts?: TopicCreateOptions): Promise<Topic> {
    return this.api.topicCreate(name, memberIds, icon, opts)
  }

  topicInfo(topicId: string): Promise<Topic> {
    return this.api.topicInfo(topicId)
  }

  topicMembers(topicId: string, limit: number = 100): Promise<TopicMembersResult> {
    return this.api.topicMembers(topicId, limit)
  }

  inviteToTopic(topicId: string, userId: string): Promise<void> {
    return this.api.topicInvite(topicId, userId)
  }

  quitTopic(topicId: string): Promise<void> {
    return this.api.topicQuit(topicId)
  }

  dismissTopic(topicId: string): Promise<void> {
    return this.api.topicDismiss(topicId)
  }

  kickFromTopic(topicId: string, userId: string): Promise<void> {
    return this.api.topicKick(topicId, userId)
  }

  setTopicNotice(topicId: string, notice: string): Promise<void> {
    return this.api.topicNotice(topicId, notice)
  }

  // ---- fallbacks / batch ----

  /** http fallback send (no ws needed); returns the server-assigned seq */
  async sendViaHttp(topicId: string, text: string): Promise<number> {
    const req: ChatRequest = {
      type: ReqType.Chat,
      chatId: randomChatId(),
      topicId: topicId,
      content: { type: ContentType.Text, text: text } as Content
    }
    const resp = await this.api.chatSend(topicId, req)
    const seq = jnum(resp, 'seq')
    const log = chatLogFromRequest(req)
    log.status = ChatLogStatus.Sent
    log.seq = seq
    this.manager.trackOutgoing(log)
    const cur = this.manager.get(topicId)
    if (cur !== undefined && seq > cur.lastSeq) {
      cur.lastSeq = seq
      cur.lastMessage = log.content
      cur.lastMessageSeq = seq
      cur.lastSenderId = this.info.userId
    }
    this.fire((): void => this.events.onLogsChanged(topicId))
    return seq
  }

  /** refresh every known conversation in one batched request */
  async batchSyncChatLogs(limit: number = 20): Promise<void> {
    const topics: Record<string, Object>[] = []
    this.conversations().forEach((c) => {
      topics.push({ 'topicId': c.topicId, 'lastSeq': c.lastSeq, 'limit': limit })
    })
    if (topics.length === 0) {
      return
    }
    const results = await this.api.chatBatchSync(topics)
    let changed = false
    for (const r of results) {
      for (const req of r.items) {
        const saved = this.manager.saveIncoming(req, this.info.userId)
        if (saved !== null) {
          changed = true
        }
        const cur = this.manager.get(r.topicId)
        if (cur !== undefined && (req.seq ?? 0) > cur.lastSeq) {
          cur.lastSeq = req.seq ?? 0
        }
      }
    }
    if (changed) {
      this.emitConversations()
    }
  }

  /** rust batchSyncChatlogs: caller-provided conversation map */
  async batchSyncChatlogs(conversations: Record<string, Conversation>, limit?: number): Promise<void> {
    const topics: Record<string, Object>[] = []
    Object.keys(conversations).forEach((topicId) => {
      topics.push({ 'topicId': topicId, 'lastSeq': conversations[topicId].lastSeq, 'limit': limit ?? 20 })
    })
    if (topics.length === 0) {
      return
    }
    const results = await this.api.chatBatchSync(topics)
    for (const r of results) {
      for (const req of r.items) {
        this.manager.saveIncoming(req, this.info.userId)
      }
    }
  }

  // ---- rust-named parity aliases ----

  getTopic(topicId: string): Promise<Topic> {
    return this.topicInfo(topicId)
  }

  getConversation(topicId: string): Conversation | undefined {
    return this.conversation(topicId)
  }

  removeTopicMember(topicId: string, userId: string): Promise<void> {
    return this.kickFromTopic(topicId, userId)
  }

  updateTopicNotice(topicId: string, text: string): Promise<void> {
    return this.setTopicNotice(topicId, text)
  }

  /** request to join a topic (rust joinTopic) */
  joinTopic(topicId: string, message: string = '', source: string = ''): Promise<void> {
    return this.api.topicKnock(topicId, message, source)
  }

  getTopicKnocks(topicId: string): Promise<TopicKnock[]> {
    return this.api.topicKnockList(topicId).then((list) => parseTopicKnocks(list))
  }

  acceptTopicJoin(topicId: string, userId: string, memo: string = ''): Promise<void> {
    return this.api.topicAcceptJoin(topicId, userId, memo)
  }

  declineTopicJoin(topicId: string, userId: string, message: string = ''): Promise<void> {
    return this.api.topicDeclineJoin(topicId, userId, message)
  }

  addTopicMember(topicId: string, userId: string): Promise<void> {
    return this.api.topicAddMember(topicId, userId)
  }

  addTopicAdmin(topicId: string, userId: string): Promise<void> {
    return this.api.topicAddAdmin(topicId, userId)
  }

  removeTopicAdmin(topicId: string, userId: string): Promise<void> {
    return this.api.topicRemoveAdmin(topicId, userId)
  }

  silentTopic(topicId: string, duration: string = ''): Promise<void> {
    return this.api.topicSilent(topicId, duration)
  }

  silentTopicMember(topicId: string, userId: string, duration: string = ''): Promise<void> {
    return this.api.topicSilentMember(topicId, userId, duration)
  }

  updateTopic(topicId: string, name: string = '', icon: string = ''): Promise<void> {
    return this.api.topicUpdate(topicId, name, icon)
  }

  /** admins of a topic resolved to full profiles */
  async getTopicAdmins(topicId: string): Promise<User[]> {
    const topic = await this.topicInfo(topicId)
    return this.getUsers(topic.admins)
  }

  async getTopicOwner(topicId: string): Promise<User | undefined> {
    const topic = await this.topicInfo(topicId)
    return this.getUser(topic.ownerId)
  }

  private quickSyncing: Map<string, Promise<SyncChatLogsResult>> = new Map()

  /** incremental sync with singleflight (rust syncChatLogsQuick) */
  syncChatLogsQuick(topicId: string, limit?: number): Promise<SyncChatLogsResult> {
    const inflight = this.quickSyncing.get(topicId)
    if (inflight !== undefined) {
      return inflight
    }
    const run = this.syncChatLogs(topicId, undefined, limit)
    this.quickSyncing.set(topicId, run)
    run.catch((): void => {})
    run.finally((): void => {
      this.quickSyncing.delete(topicId)
    })
    return run
  }

  /** full REST-backed sync + server-side read confirmation (rust syncChatLogsHeavy) */
  async syncChatLogsHeavy(topicId: string, lastSeq?: number, limit?: number): Promise<SyncChatLogsResult> {
    const r = await this.syncChatLogs(topicId, lastSeq, limit)
    await this.api.chatRead(topicId, r.lastSeq).catch((): void => {})
    return r
  }

  /** local conversation filter (rust filterConversation) */
  filterConversation(predicate: (c: Conversation) => boolean,
    lastUpdatedAt?: string, limit: number = 100): Conversation[] {
    const since = lastUpdatedAt ?? ''
    return this.conversations().filter((c) => {
      if (since !== '' && c.updatedAt >= since) {
        return false
      }
      return predicate(c)
    }).slice(0, limit)
  }

  /** first page of conversations from the server (rust syncFirstPageConversations) */
  async syncFirstPageConversations(): Promise<Conversation[]> {
    await this.syncConversations(20)
    return this.conversations()
  }

  /** runtime keepalive change (rust setKeepaliveIntervalSecs) */
  setKeepaliveIntervalSecs(secs: number): void {
    this.opts.pingIntervalSecs = secs
    this.conn.setKeepaliveIntervalSecs(secs)
  }

  /** download with progress callbacks (rust download_file); returns saved path */
  async downloadFile(fileUrl: string, cb?: DownloadCallback): Promise<string> {
    if (this.attachments === null) {
      const msg = 'media adapter not configured'
      cb?.onFail(msg)
      throw new ClientError(ClientErrorKind.Storage, msg)
    }
    const ext = fileUrl.includes('.') ? fileUrl.slice(fileUrl.lastIndexOf('.') + 1).split('?')[0] : 'bin'
    const savePath = (this.opts.tempDir !== '' ? this.opts.tempDir : '.') + '/dl_' +
      randomChatId() + randomChatId() + '.' + ext
    try {
      await this.attachments.download(fileUrl, savePath)
      cb?.onProgress(1, 1)
      cb?.onSuccess(savePath)
      return savePath
    } catch (e) {
      cb?.onFail(String(e))
      throw e as Error
    }
  }

  // ---- users (cache + profile) ----

  private async persistUser(user: User): Promise<void> {
    this.userCache.set(user.userId, user)
    this.userCachedAt.set(user.userId, nowMillis())
    if (this.userCache.size > this.opts.maxCachedUsers) {
      // evict least-recently-cached entries
      const entries: Array<[string, number]> = []
      this.userCachedAt.forEach((at, id) => entries.push([id, at]))
      entries.sort((a, b) => a[1] - b[1])
      const excess = this.userCache.size - this.opts.maxCachedUsers
      for (let i = 0; i < excess && i < entries.length; i++) {
        this.userCache.delete(entries[i][0])
        this.userCachedAt.delete(entries[i][0])
      }
    }
    if (this.userStore !== null) {
      this.userStore.set('__user', user.userId, nowMillis(), JSON.stringify(user)).catch((): void => {})
    }
  }

  /**
   * cached profile; when the cache is stale, `blocking` decides whether to
   * refresh from the server (rust get_user semantics)
   */
  async getUser(userId: string, blocking: boolean = true): Promise<User | undefined> {
    const cachedAt = this.userCachedAt.get(userId) ?? 0
    const fresh = nowMillis() - cachedAt < this.opts.userCacheExpireSecs * 1000
    const cached = this.userCache.get(userId)
    if (cached !== undefined && fresh) {
      return cached
    }
    if (this.userStore !== null && !this.userCache.has(userId)) {
      const row = await this.userStore.get('__user', userId).catch((): string | undefined => undefined)
      if (row !== undefined) {
        const rowJson = parseJson(row)
        if (rowJson !== null) {
          const user = parseUser(rowJson)
          this.userCache.set(userId, user)
          this.userCachedAt.set(userId, nowMillis())
        }
      }
    }
    const cached2 = this.userCache.get(userId)
    if (!blocking && cached2 !== undefined) {
      return cached2
    }
    try {
      const user = await this.api.getProfile(userId)
      await this.persistUser(user)
      return user
    } catch (e) {
      return cached2
    }
  }

  async getUsers(userIds: string[]): Promise<User[]> {
    const missing: string[] = []
    const out: User[] = []
    for (const id of userIds) {
      const u = await this.getUser(id, false)
      if (u !== undefined) {
        out.push(u)
      } else {
        missing.push(id)
      }
    }
    if (missing.length > 0) {
      const fetched = await this.api.getProfiles(missing)
      for (const u of fetched) {
        await this.persistUser(u)
        out.push(u)
      }
    }
    return out
  }

  async setUserRemark(userId: string, remark: string): Promise<void> {
    await this.api.setUserRemark(userId, remark)
  }

  async setUserStar(userId: string, star: boolean): Promise<void> {
    await this.api.setUserStar(userId, star)
  }

  async setUserBlock(userId: string, blocked: boolean): Promise<void> {
    await this.api.setUserBlock(userId, blocked)
  }

  async setAllowGuestChat(allow: boolean): Promise<void> {
    await this.api.setAllowGuestChat(allow)
  }

  // ---- misc parity ----

  getLastAliveAt(): number {
    return this.conn.lastAliveAt
  }

  /** internal sizes for leak monitoring (soak tests / diagnostics) */
  debugStats(): Record<string, number> {
    const out: Record<string, number> = {}
    out['pending'] = this.queue.size()
    out['conversations'] = this.manager.size()
    out['logTopics'] = this.manager.logTopics()
    out['logEntries'] = this.manager.logEntries()
    out['removedTombstones'] = this.manager.tombstones()
    out['mappedChatIds'] = this.logTopicByChatId.size
    out['cachedUsers'] = this.userCache.size
    return out
  }

  /** host-customizable unread counting (rust setCountableCallback) */
  setCountableCallback(fn: (content: Content) => boolean): void {
    this.countable = fn
  }

  /** manual keepalive frame (parity doPing) */
  doPing(cb?: SendCallback): string {
    const req: ChatRequest = {
      type: ReqType.Ping,
      chatId: randomChatId(),
      content: { type: 'ping', text: JSON.stringify({ timestamp: nowMillis(), 'error_logs': [] }) } as Content
    }
    this.queue.offer(req, cb)
    return req.chatId ?? ''
  }

  /** ws-only read receipt (rust doRead) */
  doRead(topicId: string, seq?: number): void {
    const target = seq ?? this.conversation(topicId)?.lastSeq ?? 0
    if (target <= 0) {
      return
    }
    this.manager.markLocallyRead(topicId, target)
    const frame: ChatRequest = { type: ReqType.Read, topicId: topicId, seq: target }
    this.conn.send(JSON.stringify(frame))
  }

  /** mark a conversation unread (server + local) */
  async markConversationUnread(topicId: string): Promise<void> {
    await this.api.chatUnread(topicId, true)
    const cur = this.manager.get(topicId)
    if (cur !== undefined) {
      cur.unread = Math.max(cur.unread, 1)
    }
    this.emitConversations()
  }

  /** clear every message of a topic on the server and locally */
  async cleanMessages(topicId: string): Promise<void> {
    await this.api.chatClearMessages(topicId)
    this.manager.clearLogs(topicId)
    this.fire((): void => this.events.onLogsChanged(topicId))
  }

  /** remove selected messages; syncToServer=false only affects local logs */
  async removeMessages(topicId: string, chatIds: string[], syncToServer: boolean = true): Promise<void> {
    for (const id of chatIds) {
      this.manager.removeLog(topicId, id)
    }
    if (syncToServer) {
      await this.api.chatRemoveMessages(topicId, chatIds)
    }
    this.fire((): void => this.events.onLogsChanged(topicId))
  }

  /** local-only alias of cleanMessages (rust clearConversation) */
  clearConversation(topicId: string): void {
    this.manager.clearLogs(topicId)
    this.fire((): void => this.events.onLogsChanged(topicId))
  }

  getChatLog(topicId: string, chatId: string): ChatLog | undefined {
    return this.manager.findLog(topicId, chatId)
  }

  saveChatLogs(logs: ChatLog[]): void {
    this.manager.saveLogs(logs)
  }

  /** local search across stored logs (rust searchChatLog) */
  searchChatLog(topicId: string | undefined, senderId: string | undefined, keyword: string): GetChatLogsResult {
    const kw = keyword.toLowerCase()
    const out: ChatLog[] = []
    const scan = (list: ChatLog[]): void => {
      for (const l of list) {
        if (topicId !== undefined && l.topicId !== topicId) {
          continue
        }
        if (senderId !== undefined && l.senderId !== senderId) {
          continue
        }
        if (kw !== '' && !(l.content.text ?? '').toLowerCase().includes(kw)) {
          continue
        }
        out.push(l)
      }
    }
    if (topicId !== undefined) {
      scan(this.getLogs(topicId, Number.MAX_SAFE_INTEGER))
    } else {
      this.conversations().forEach((c) => {
        scan(this.getLogs(c.topicId, Number.MAX_SAFE_INTEGER))
      })
    }
    const lastSeq = out.length > 0 ? out[0].seq : 0
    const updatedAt = out.length > 0 ? out[0].createdAt : ''
    const result: GetChatLogsResult = {
      topicId: topicId ?? '',
      hasMore: false,
      updatedAt: updatedAt,
      lastSeq: lastSeq,
      items: out
    }
    return result
  }

  /** one history page, newest first (rust fetchChatLogsDesc) */
  async fetchChatLogsDesc(topicId: string, lastSeq?: number, limit?: number): Promise<SyncChatLogsResult> {
    return this.syncChatLogs(topicId, lastSeq ?? 0, limit ?? this.opts.maxSyncLogsLimit)
  }

  /** ws read receipt only; heavy=true also confirms via REST */
  async setConversationRead(topicId: string, seq?: number, heavy: boolean = false): Promise<void> {
    const cur = this.manager.get(topicId)
    const target = seq ?? cur?.lastSeq ?? 0
    if (target <= 0) {
      return
    }
    this.manager.markLocallyRead(topicId, target)
    const frame: ChatRequest = { type: ReqType.Read, topicId: topicId, seq: target }
    this.conn.send(JSON.stringify(frame))
    if (heavy) {
      await this.api.chatRead(topicId, target)
    }
    this.emitConversations()
  }

  async setAllConversationsRead(): Promise<void> {
    await this.api.chatReadAll()
    this.conversations().forEach((c) => {
      this.manager.markLocallyRead(c.topicId, c.lastSeq)
    })
    this.emitConversations()
  }

  async setConversationTags(topicId: string, tags: Tag[]): Promise<Conversation | undefined> {
    const fields: Record<string, Object> = {}
    fields['tags'] = tags
    await this.api.chatUpdate(topicId, fields)
    this.manager.updateFields(topicId, fields)
    return this.conversation(topicId)
  }

  /** send one raw request over http (rust sendChatRequest) */
  async sendChatRequest(topicId: string, req: ChatRequest): Promise<ApiSendResponse> {
    const json = await this.api.chatSend(topicId, req)
    return parseApiSendResponse(json)
  }

  /** send one raw request over the ws queue (rust sendChatRequestViaConnection) */
  sendChatRequestViaConnection(req: ChatRequest, cb?: SendCallback): string {
    this.queue.offer(req, cb)
    return req.chatId ?? ''
  }

  /** drop a queued/unacked send (rust cancelSend) */
  cancelSend(chatId: string): void {
    if (this.queue.cancel(chatId)) {
      const topicId = this.logTopicByChatId.get(chatId)
      if (topicId !== undefined) {
        const l = this.manager.findLog(topicId, chatId)
        if (l !== undefined) {
          l.status = ChatLogStatus.SendFailed
        }
        this.fire((): void => this.events.onLogsChanged(topicId))
      }
    }
  }
}

function formatDuration(totalSecs: number): string {
  const s = Math.max(0, Math.floor(totalSecs))
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const sec = s % 60
  const two = (n: number): string => n < 10 ? '0' + String(n) : String(n)
  return `${two(h)}:${two(m)}:${two(sec)}`
}
