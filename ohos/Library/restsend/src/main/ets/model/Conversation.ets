import { Content, parseContent } from './Content'
import { Extra } from './User'
import { Json, jstr, jnum, jbool, jobj, jarr } from '../common/Json'

export interface Tag {
  id: string
  tagType: string
  label: string
}

function parseTags(o: Json): Tag[] {
  const tags: Tag[] = []
  for (const t of jarr(o, 'tags')) {
    const tag: Tag = { id: jstr(t, 'id'), tagType: jstr(t, 'tagType'), label: jstr(t, 'label') }
    tags.push(tag)
  }
  return tags
}

/**
 * Conversation row. Field names match the wire format (camelCase)
 * so JSON round-trips work without renaming.
 */
export interface Conversation {
  ownerId: string
  topicId: string
  updatedAt: string
  startSeq: number
  lastSeq: number
  lastReadSeq: number
  lastReadAt?: string
  multiple: boolean
  attendee: string
  members: number
  name: string
  icon: string
  kind: string
  sticky: boolean
  mute: boolean
  source: string
  unread: number
  lastSenderId: string
  lastMessage?: Content
  lastMessageAt: string
  lastMessageSeq?: number
  remark?: string
  extra?: Extra
  topicExtra?: Extra
  topicOwnerId?: string
  topicCreatedAt?: string
  tags?: Tag[]
  cachedAt: number
  isPartial: boolean
}

export function parseConversation(o: Json): Conversation {
  const c: Conversation = {
    ownerId: jstr(o, 'ownerId'),
    topicId: jstr(o, 'topicId'),
    updatedAt: jstr(o, 'updatedAt'),
    startSeq: jnum(o, 'startSeq'),
    lastSeq: jnum(o, 'lastSeq'),
    lastReadSeq: jnum(o, 'lastReadSeq'),
    lastReadAt: jstr(o, 'lastReadAt'),
    multiple: jbool(o, 'multiple'),
    attendee: jstr(o, 'attendee'),
    members: jnum(o, 'members'),
    name: jstr(o, 'name'),
    icon: jstr(o, 'icon'),
    kind: jstr(o, 'kind'),
    sticky: jbool(o, 'sticky'),
    mute: jbool(o, 'mute'),
    source: jstr(o, 'source'),
    unread: jnum(o, 'unread'),
    lastSenderId: jstr(o, 'lastSenderId'),
    lastMessageAt: jstr(o, 'lastMessageAt'),
    lastMessageSeq: jnum(o, 'lastMessageSeq'),
    remark: jstr(o, 'remark'),
    topicOwnerId: jstr(o, 'topicOwnerId'),
    topicCreatedAt: jstr(o, 'topicCreatedAt'),
    tags: parseTags(o),
    cachedAt: jnum(o, 'cachedAt'),
    isPartial: jbool(o, 'isPartial')
  }
  const lastMessage = jobj(o, 'lastMessage')
  if (lastMessage !== undefined) {
    c.lastMessage = parseContent(lastMessage)
  }
  const extra = jobj(o, 'extra')
  if (extra !== undefined) {
    c.extra = extra
  }
  const topicExtra = jobj(o, 'topicExtra')
  if (topicExtra !== undefined) {
    c.topicExtra = topicExtra
  }
  return c
}

export function conversationToJson(c: Conversation): Json {
  return JSON.parse(JSON.stringify(c)) as Json
}
