import { User } from './User'
import { Content, parseContent } from './Content'
import { Json, jstr, jnum, jbool, jobj, jstrArr } from '../common/Json'

/** WS message types, mirror of the Rust `ChatRequestType`. */
export enum ReqType {
  Chat = 'chat',
  Typing = 'typing',
  Read = 'read',
  Resp = 'resp',
  Kickout = 'kickout',
  /** the backend's force-offline frame (same meaning as kickout) */
  Kick = 'kick',
  /** recall frames sent as top-level type (chat-log style) */
  Recall = 'recall',
  System = 'system',
  Nop = 'nop',
  Ping = 'ping'
}

/**
 * One websocket frame. Field names match the wire format directly
 * so JSON.stringify(model) produces a valid payload.
 */
export interface ChatRequest {
  type: string
  chatId?: string
  code?: number
  topicId?: string
  seq?: number
  attendee?: string
  attendeeProfile?: User
  createdAt?: string
  content?: Content
  e2eContent?: string
  message?: string
  source?: string
  mentions?: string[]
  mentionAll?: boolean
  reply?: string
  replyContent?: string
  /** present in chat-log shaped payloads */
  recall?: boolean
}

export function parseChatRequest(o: Json): ChatRequest {
  const req: ChatRequest = {
    type: jstr(o, 'type'),
    chatId: jstr(o, 'chatId', jstr(o, 'id')),
    code: jnum(o, 'code'),
    topicId: jstr(o, 'topicId'),
    seq: jnum(o, 'seq'),
    // chat-log shaped payloads use senderId
    attendee: jstr(o, 'attendee', jstr(o, 'senderId')),
    createdAt: jstr(o, 'createdAt', jstr(o, 'sentAt', jstr(o, 'cachedAt'))),
    e2eContent: jstr(o, 'e2eContent'),
    message: jstr(o, 'message'),
    source: jstr(o, 'source'),
    mentions: jstrArr(o, 'mentions'),
    mentionAll: jbool(o, 'mentionAll'),
    reply: jstr(o, 'reply'),
    replyContent: jstr(o, 'replyContent'),
    recall: jbool(o, 'recall')
  }
  const profile = jobj(o, 'attendeeProfile')
  if (profile !== undefined) {
    req.attendeeProfile = { userId: jstr(profile, 'userId'), name: jstr(profile, 'name'), avatar: jstr(profile, 'avatar') }
  }
  const content = jobj(o, 'content')
  if (content !== undefined) {
    req.content = parseContent(content)
  }
  return req
}

export function parseChatRequestText(text: string): ChatRequest | null {
  const parsed = JSON.parse(text) as Json | null
  if (parsed === null) {
    return null
  }
  return parseChatRequest(parsed)
}
