import { Json, jstr, jnum, jarr, jbool } from '../common/Json'
import { ChatLog, parseChatLog } from './ChatLog'

/** topic join request (admin list), rust TopicKnock parity */
export interface TopicKnock {
  id: string
  topicId: string
  userId: string
  memo: string
  createdAt: string
}

export function parseTopicKnock(o: Json): TopicKnock {
  // backend wire is snake_case: {created_at, topic_id, user_id, message, status, ...}
  const k: TopicKnock = {
    id: jstr(o, 'id', jstr(o, 'user_id', jstr(o, 'attendee'))),
    topicId: jstr(o, 'topicId', jstr(o, 'topic_id')),
    userId: jstr(o, 'userId', jstr(o, 'user_id', jstr(o, 'attendee'))),
    memo: jstr(o, 'memo', jstr(o, 'message')),
    createdAt: jstr(o, 'createdAt', jstr(o, 'created_at'))
  }
  return k
}

/** list_knock may return a bare array or {items:[...]} */
export function parseTopicKnocks(json: Json | Json[]): TopicKnock[] {
  const out: TopicKnock[] = []
  const raw: Json[] = Array.isArray(json) ? json : jarr(json, 'items')
  for (const item of raw) {
    out.push(parseTopicKnock(item))
  }
  return out
}

/** rust GetChatLogsResult parity (search / history pages) */
export interface GetChatLogsResult {
  topicId: string
  hasMore: boolean
  updatedAt: string
  lastSeq: number
  items: ChatLog[]
}

export function parseGetChatLogsResult(topicId: string, o: Json): GetChatLogsResult {
  const logs: ChatLog[] = []
  for (const item of jarr(o, 'items')) {
    const parsed = parseChatLog(item)
    if (parsed !== null) {
      logs.push(parsed)
    }
  }
  const r: GetChatLogsResult = {
    topicId: jstr(o, 'topicId', topicId),
    hasMore: jbool(o, 'hasMore'),
    updatedAt: jstr(o, 'updatedAt'),
    lastSeq: jnum(o, 'lastSeq'),
    items: logs
  }
  return r
}

/** rust ApiSendResponse parity (http fallback send result) */
export interface ApiSendResponse {
  senderId: string
  topicId: string
  attendeeId: string
  chatId: string
  code: number
  seq: number
  message: string
  usage: number
}

export function parseApiSendResponse(o: Json): ApiSendResponse {
  const r: ApiSendResponse = {
    senderId: jstr(o, 'senderId'),
    topicId: jstr(o, 'topicId'),
    attendeeId: jstr(o, 'attendeeId'),
    chatId: jstr(o, 'chatId'),
    code: jnum(o, 'code'),
    seq: jnum(o, 'seq'),
    message: jstr(o, 'message'),
    usage: jnum(o, 'usage')
  }
  return r
}

