import { Content, ContentType, newContent, parseContent } from './Content'
import { ChatRequest } from './ChatRequest'
import { Json, jstr, jnum, jbool, jobj } from '../common/Json'
import { nowMillis } from '../common/Ids'

/** Mirror of the Rust `ChatLogStatus`. */
export enum ChatLogStatus {
  Uploading = 'uploading',
  Sending = 'sending',
  Sent = 'sent',
  Downloading = 'downloading',
  Received = 'received',
  Read = 'read',
  SendFailed = 'sendFailed'
}

export interface ChatLog {
  topicId: string
  id: string
  seq: number
  createdAt: string
  senderId: string
  content: Content
  read: boolean
  recall: boolean
  status: ChatLogStatus
  cachedAt: number
}

export function parseChatLog(o: Json): ChatLog {
  const contentJson = jobj(o, 'content')
  const log: ChatLog = {
    topicId: jstr(o, 'topicId'),
    id: jstr(o, 'id'),
    seq: jnum(o, 'seq'),
    createdAt: jstr(o, 'createdAt', jstr(o, 'sentAt', jstr(o, 'cachedAt'))),
    senderId: jstr(o, 'senderId', jstr(o, 'attendee')),
    content: contentJson !== undefined ? parseContent(contentJson) : ({ type: '' } as Content),
    read: jbool(o, 'read'),
    recall: jbool(o, 'recall'),
    status: jstr(o, 'status') as ChatLogStatus,
    cachedAt: jnum(o, 'cachedAt', nowMillis())
  }
  return log
}

export function chatLogFromRequest(req: ChatRequest): ChatLog {
  const log: ChatLog = {
    topicId: req.topicId ?? '',
    id: req.chatId ?? '',
    seq: req.seq ?? 0,
    createdAt: req.createdAt ?? '',
    senderId: req.attendee ?? '',
    content: req.content ?? newContent(ContentType.None),
    read: false,
    recall: req.type === 'recall' || (req.recall ?? false),
    status: ChatLogStatus.Received,
    cachedAt: nowMillis()
  }
  return log
}
