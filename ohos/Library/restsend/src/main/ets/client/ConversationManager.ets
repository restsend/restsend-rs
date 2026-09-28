import { Conversation, Tag, parseConversation } from '../model/Conversation'
import { ChatLog, ChatLogStatus, chatLogFromRequest, parseChatLog } from '../model/ChatLog'
import { ChatRequest, ReqType } from '../model/ChatRequest'
import { Content, ContentType, parseContent } from '../model/Content'
import { Json, parseJson, jstr, jnum, jbool, jobj } from '../common/Json'
import { nowMillis } from '../common/Ids'
import { Table } from '../store/Table'

/** content types that increment the unread badge */
export function isCountableContent(type: string): boolean {
  return type === ContentType.Text || type === ContentType.Image || type === ContentType.Video ||
    type === ContentType.Voice || type === ContentType.File || type === ContentType.Location ||
    type === ContentType.Link || type === ContentType.Sticker || type === ContentType.Invite ||
    type === ContentType.Contact
}

function laterUpdatedAt(a: string, b: string): string {
  return a >= b ? a : b
}

/**
 * In-memory conversation list + chat logs, with the merge semantics of the
 * Rust core: stale snapshot guard, local read-state protection, extra merge
 * by key, lastMessage healing and removed-conversation anti-refill.
 */
interface RemovedTombstone {
  at: number
  seq: number
}

export class ConversationManager {
  private conversations: Map<string, Conversation> = new Map()
  private logs: Map<string, ChatLog[]> = new Map()
  /** topicId -> removal time + seq; messages at/below seq are dropped until TTL */
  private removed: Map<string, RemovedTombstone> = new Map()
  /** topics whose read state was advanced locally */
  private localRead: Set<string> = new Set()
  /** topics whose logs were loaded back from storage */
  private loadedTopics: Set<string> = new Set()
  private hydrated = false
  private store: Table | null
  private maxLogsPerTopic: number

  constructor(store: Table | null, maxLogsPerTopic: number) {
    this.store = store
    this.maxLogsPerTopic = maxLogsPerTopic
  }

  /**
   * Load conversations (and recent logs of the newest ones) from storage.
   * Call once before the first server sync so a restart shows data offline.
   */
  async hydrate(maxLogTopics: number = 50): Promise<void> {
    if (this.hydrated || this.store === null) {
      this.hydrated = true
      return
    }
    this.hydrated = true
    const rows = await this.store.query('', 1000, true)
    const conversations: Conversation[] = []
    for (const row of rows) {
      const parsed = parseJson(row.value)
      if (parsed === null) {
        continue
      }
      const c = parseConversation(parsed)
      if (c.topicId !== '' && !this.conversations.has(c.topicId)) {
        this.conversations.set(c.topicId, c)
        conversations.push(c)
      }
    }
    // hydrate logs of the most recent conversations only (bounded work)
    this.sort(conversations)
    const recent = conversations.slice(0, maxLogTopics)
    for (const c of recent) {
      await this.ensureLogsLoaded(c.topicId)
    }
  }

  private async ensureLogsLoaded(topicId: string): Promise<void> {
    if (this.store === null || this.loadedTopics.has(topicId)) {
      return
    }
    this.loadedTopics.add(topicId)
    const rows = await this.store.query(topicId, this.maxLogsPerTopic, true)
    let list = this.logs.get(topicId)
    if (list === undefined) {
      list = []
      this.logs.set(topicId, list)
    }
    for (const row of rows) {
      const parsed = parseJson(row.value)
      if (parsed === null) {
        continue
      }
      const log = parseChatLog(parsed)
      const known = list.some((l) => l.id === log.id)
      if (!known && log.id !== '') {
        list.push(log)
      }
    }
    list.sort((a, b) => b.seq - a.seq)
  }

  get(topicId: string): Conversation | undefined {
    return this.conversations.get(topicId)
  }

  all(): Conversation[] {
    const list: Conversation[] = []
    this.conversations.forEach((c) => list.push(c))
    return this.sort(list)
  }

  sort(list: Conversation[]): Conversation[] {
    list.sort((a, b) => {
      if (a.sticky !== b.sticky) {
        return a.sticky ? -1 : 1
      }
      return b.updatedAt.localeCompare(a.updatedAt)
    })
    return list
  }

  totalUnread(): number {
    let n = 0
    this.conversations.forEach((c) => {
      if (c.unread > 0) {
        n += c.unread
      }
    })
    return n
  }

  size(): number {
    return this.conversations.size
  }

  logTopics(): number {
    return this.logs.size
  }

  logEntries(): number {
    let n = 0
    this.logs.forEach((list) => {
      n += list.length
    })
    return n
  }

  tombstones(): number {
    return this.removed.size
  }

  markLocallyRead(topicId: string, seq: number): void {
    const c = this.conversations.get(topicId)
    if (c === undefined) {
      return
    }
    this.localRead.add(topicId)
    if (seq > c.lastReadSeq) {
      c.lastReadSeq = seq
    }
    c.unread = Math.max(0, c.lastSeq - c.lastReadSeq)
    this.persistConversation(c)
  }

  markRemoved(topicId: string, seq: number): void {
    const cur = this.removed.get(topicId)
    if (cur === undefined || seq > cur.seq) {
      const tomb: RemovedTombstone = { at: nowMillis(), seq: seq }
      this.removed.set(topicId, tomb)
    }
    this.conversations.delete(topicId)
    this.logs.delete(topicId)
    this.localRead.delete(topicId)
    if (this.store !== null) {
      this.store.remove('', topicId).catch((): void => {})
      this.store.clearPartition(topicId).catch((): void => {})
    }
  }

  isRemovedStale(topicId: string, seq: number): boolean {
    const cur = this.removed.get(topicId)
    return cur !== undefined && seq <= cur.seq
  }

  /** drop expired removal tombstones (rust removedConversationCache parity) */
  expireRemovedTombstones(ttlSecs: number): void {
    const cutoff = nowMillis() - ttlSecs * 1000
    const expired: string[] = []
    this.removed.forEach((v, topicId) => {
      if (v.at < cutoff) {
        expired.push(topicId)
      }
    })
    expired.forEach((topicId) => this.removed.delete(topicId))
  }

  /** apply REST-side conversation updates (sticky/mute/remark/tags/extra) */
  updateFields(topicId: string, fields: Record<string, Object>): boolean {
    const cur = this.conversations.get(topicId)
    if (cur === undefined) {
      return false
    }
    const incoming: Conversation = JSON.parse(JSON.stringify(cur)) as Conversation
    if (typeof fields['sticky'] === 'boolean') {
      incoming.sticky = fields['sticky'] as boolean
    }
    if (typeof fields['mute'] === 'boolean') {
      incoming.mute = fields['mute'] as boolean
    }
    if (typeof fields['remark'] === 'string') {
      incoming.remark = fields['remark'] as string
    }
    const tags = fields['tags']
    if (tags !== undefined && Array.isArray(tags)) {
      const copy: Tag[] = []
      for (const t of tags as Array<Object>) {
        const o = t as Record<string, Object>
        const tag: Tag = {
          id: typeof o['id'] === 'string' ? o['id'] as string : '',
          tagType: typeof o['tagType'] === 'string' ? o['tagType'] as string : '',
          label: typeof o['label'] === 'string' ? o['label'] as string : ''
        }
        copy.push(tag)
      }
      incoming.tags = copy
    }
    const extra = fields['extra']
    if (extra !== undefined && typeof extra === 'object' && !Array.isArray(extra)) {
      const map = extra as Record<string, Object>
      const merged: Record<string, Object> = {}
      if (cur.extra !== undefined) {
        Object.keys(cur.extra).forEach((k) => {
          merged[k] = cur.extra![k]
        })
      }
      Object.keys(map).forEach((k) => {
        merged[k] = map[k]
      })
      incoming.extra = merged
    }
    this.conversations.set(topicId, incoming)
    this.persistConversation(incoming)
    return true
  }

  /** periodic housekeeping: prune conversations and expire old logs */
  maintain(maxConversations: number, retentionDays: number, removedTtlSecs: number = 300): void {
    this.expireRemovedTombstones(removedTtlSecs)
    const cutoff = nowMillis() - retentionDays * 86400000
    // expire old logs
    this.logs.forEach((list, topicId) => {
      const kept = list.filter((l) => l.cachedAt >= cutoff)
      if (kept.length !== list.length) {
        this.logs.set(topicId, kept)
        if (this.store !== null) {
          const doomed = list.filter((l) => l.cachedAt < cutoff)
          doomed.forEach((l) => {
            this.store!.remove(topicId, l.id).catch((): void => {})
          })
        }
      }
    })
    // prune oldest conversations beyond the limit
    const all = this.all()
    if (all.length > maxConversations) {
      all.slice(maxConversations).forEach((c) => {
        this.markRemoved(c.topicId, c.lastSeq)
      })
    }
  }

  /** merge one server-side conversation snapshot; returns true when changed */
  merge(incoming: Conversation): boolean {
    if (incoming.topicId === '') {
      return false
    }
    const cur = this.conversations.get(incoming.topicId)
    if (cur === undefined) {
      incoming.cachedAt = nowMillis()
      this.conversations.set(incoming.topicId, incoming)
      this.persistConversation(incoming)
      return true
    }
    // stale snapshot guard (except for the fresher read state kept below)
    if (incoming.updatedAt !== '' && cur.updatedAt !== '' && incoming.updatedAt < cur.updatedAt) {
      return false
    }
    // local read-state protection: do not resurrect unread we cleared
    if (this.localRead.has(incoming.topicId) && incoming.lastSeq <= cur.lastSeq) {
      incoming.lastReadSeq = Math.max(incoming.lastReadSeq, cur.lastReadSeq)
    }
    incoming.lastReadSeq = Math.max(incoming.lastReadSeq, cur.lastReadSeq)
    incoming.unread = Math.max(0, incoming.lastSeq - incoming.lastReadSeq)
    incoming.cachedAt = nowMillis()
    incoming.extra = mergeExtra(cur.extra, incoming.extra)
    this.conversations.set(incoming.topicId, incoming)
    this.persistConversation(incoming)
    return true
  }

  /** `conversation.update` push: heal lastMessage from the authoritative snapshot */
  applyServerSnapshot(chat: ChatRequest): boolean {
    const topicId = chat.topicId ?? ''
    if (topicId === '' || chat.content?.text === undefined) {
      return false
    }
    const payload = parseJson(chat.content.text)
    if (payload === null) {
      return false
    }
    const cur = this.conversations.get(topicId)
    if (cur === undefined) {
      return false
    }
    const incoming: Conversation = JSON.parse(JSON.stringify(cur)) as Conversation
    const lastSeq = jnum(payload, 'lastSeq')
    if (lastSeq > 0) {
      incoming.lastSeq = lastSeq
    }
    incoming.unread = jnum(payload, 'unread', cur.unread)
    incoming.sticky = jbool(payload, 'sticky', cur.sticky)
    incoming.mute = jbool(payload, 'mute', cur.mute)
    incoming.lastSenderId = jstr(payload, 'lastSenderId', cur.lastSenderId)
    incoming.lastMessageSeq = jnum(payload, 'lastMessageSeq', cur.lastMessageSeq ?? 0)
    const lastMessage = jobj(payload, 'lastMessage')
    if (lastMessage !== undefined) {
      incoming.lastMessage = parseContentJson(lastMessage)
    }
    if (this.localRead.has(topicId)) {
      incoming.lastReadSeq = Math.max(incoming.lastReadSeq, cur.lastReadSeq)
      incoming.unread = Math.max(0, incoming.lastSeq - incoming.lastReadSeq)
    }
    const changed = incoming.updatedAt >= cur.updatedAt || incoming.lastSeq >= cur.lastSeq
    if (changed) {
      incoming.updatedAt = laterUpdatedAt(incoming.updatedAt, cur.updatedAt)
      this.conversations.set(topicId, incoming)
      this.persistConversation(incoming)
    }
    return changed
  }

  // ---- chat logs ----

  getLogs(topicId: string, limit: number): ChatLog[] {
    const list = this.logs.get(topicId)
    if (list === undefined) {
      return []
    }
    return list.slice(0, limit)
  }

  /** highest local seq for a topic (0 when nothing stored) */
  topSeq(topicId: string): number {
    const list = this.logs.get(topicId)
    if (list === undefined || list.length === 0) {
      return 0
    }
    return list[0].seq
  }

  /** lowest local seq for a topic (0 when nothing stored) */
  minSeq(topicId: string): number {
    const list = this.logs.get(topicId)
    if (list === undefined) {
      return 0
    }
    for (let i = list.length - 1; i >= 0; i--) {
      if (list[i].seq > 0) {
        return list[i].seq
      }
    }
    return 0
  }

  /**
   * Rust-parity lag detection: sum of adjacent seq differences exceeds the
   * entry count -> there is a hole in the middle (e.g. messages missed while
   * the conversation snapshot already advanced past them).
   */
  hasSeqHole(topicId: string): boolean {
    const list = this.logs.get(topicId)
    if (list === undefined || list.length < 2) {
      return false
    }
    let diff = 0
    let count = 0
    let prev = -1
    for (const l of list) {
      if (l.seq <= 0) {
        continue
      }
      if (prev > 0) {
        diff += prev - l.seq
        count += 1
      }
      prev = l.seq
    }
    return diff > count
  }

  findLog(topicId: string, chatId: string): ChatLog | undefined {
    const list = this.logs.get(topicId)
    if (list === undefined) {
      return undefined
    }
    for (const l of list) {
      if (l.id === chatId) {
        return l
      }
    }
    return undefined
  }

  /**
   * Save an incoming chat frame. Handles chatId dedup, own-echo upgrade
   * (sending -> sent) and the removed-conversation guard.
   */
  saveIncoming(chat: ChatRequest, selfId: string): ChatLog | null {
    const topicId = chat.topicId ?? ''
    const chatId = chat.chatId ?? ''
    if (topicId === '' || chatId === '') {
      return null
    }
    const seq = chat.seq ?? 0
    if (this.isRemovedStale(topicId, seq)) {
      return null
    }
    const existing = this.findLog(topicId, chatId)
    if (existing !== undefined) {
      if (existing.status === ChatLogStatus.Sending && chat.attendee === selfId) {
        existing.status = ChatLogStatus.Sent
        if (seq > 0) {
          existing.seq = seq
        }
        return existing
      }
      return null // duplicated frame
    }
    const log = chatLogFromRequest(chat)
    if (chat.attendee === selfId) {
      log.status = ChatLogStatus.Sent
    }
    this.insertLog(topicId, log)
    return log
  }

  /** register an outgoing message before the server assigns a seq */
  trackOutgoing(log: ChatLog): void {
    this.insertLog(log.topicId, log)
  }

  /** drop one local log (memory + storage) */
  removeLog(topicId: string, chatId: string): void {
    const list = this.logs.get(topicId)
    if (list === undefined) {
      return
    }
    const kept = list.filter((l) => l.id !== chatId)
    this.logs.set(topicId, kept)
    if (this.store !== null) {
      this.store.remove(topicId, chatId).catch((): void => {})
    }
  }

  /** drop every local log of a topic (memory + storage) */
  clearLogs(topicId: string): void {
    this.logs.delete(topicId)
    if (this.store !== null) {
      this.store.clearPartition(topicId).catch((): void => {})
    }
  }

  /** bulk upsert (e.g. import / saveChatLogs) */
  saveLogs(logs: ChatLog[]): void {
    for (const log of logs) {
      const existing = this.findLog(log.topicId, log.id)
      if (existing === undefined) {
        this.insertLog(log.topicId, log)
      }
    }
  }

  private insertLog(topicId: string, log: ChatLog): void {
    let list = this.logs.get(topicId)
    if (list === undefined) {
      list = []
      this.logs.set(topicId, list)
    }
    list.unshift(log)
    list.sort((a, b) => b.seq - a.seq)
    if (list.length > this.maxLogsPerTopic) {
      list.length = this.maxLogsPerTopic
    }
    if (this.store !== null) {
      this.store.set(topicId, log.id, log.seq, JSON.stringify(log)).catch((): void => {})
    }
  }

  /** write-through one conversation snapshot (partition '', sort = updatedAt ms) */
  private persistConversation(c: Conversation): void {
    if (this.store === null) {
      return
    }
    const ts = Date.parse(c.updatedAt)
    const sortBy = Number.isNaN(ts) ? c.cachedAt : ts
    this.store.set('', c.topicId, sortBy, JSON.stringify(c)).catch((): void => {})
  }
}

function parseContentJson(o: Json): Content {
  const c: Content = { type: jstr(o, 'type'), text: jstr(o, 'text') }
  return c
}

/** merge extra maps key by key, incoming wins */
function mergeExtra(base: Record<string, Object> | undefined, incoming: Record<string, Object> | undefined): Record<string, Object> | undefined {
  if (base === undefined) {
    return incoming
  }
  if (incoming === undefined) {
    return base
  }
  const merged: Record<string, Object> = {}
  Object.keys(base).forEach((k) => {
    merged[k] = base[k]
  })
  Object.keys(incoming).forEach((k) => {
    merged[k] = incoming[k]
  })
  return merged
}
