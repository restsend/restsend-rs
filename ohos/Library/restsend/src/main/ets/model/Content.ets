import { Extra } from './User'
import { Json, jstr, jnum, jbool, jobj, jstrArr } from '../common/Json'

/** Content type strings, mirror of the Rust `ContentType`. */
export enum ContentType {
  None = '',
  Text = 'text',
  Image = 'image',
  Video = 'video',
  Voice = 'voice',
  File = 'file',
  Location = 'location',
  Sticker = 'sticker',
  Contact = 'contact',
  Link = 'link',
  Invite = 'invite',
  Logs = 'logs',
  Recall = 'recall',
  Recalled = 'recalled',
  TopicCreate = 'topic.create',
  TopicDismiss = 'topic.dismiss',
  TopicQuit = 'topic.quit',
  TopicKickout = 'topic.kickout',
  TopicJoin = 'topic.join',
  TopicNotice = 'topic.notice',
  TopicUpdate = 'topic.update',
  TopicKnock = 'topic.knock',
  TopicKnockAccept = 'topic.knock.accept',
  TopicKnockReject = 'topic.knock.reject',
  TopicSilent = 'topic.silent',
  TopicSilentMember = 'topic.silent.member',
  TopicChangeOwner = 'topic.changeowner',
  ConversationUpdate = 'conversation.update',
  ConversationRemoved = 'conversation.removed',
  UpdateExtra = 'update.extra'
}

/** Attachment status, mirror of the Rust `AttachmentStatus`. */
export enum AttachmentStatus {
  ToUpload = 'toUpload',
  ToDownload = 'toDownload',
  Uploading = 'uploading',
  Downloading = 'downloading',
  Paused = 'paused',
  Done = 'done',
  Failed = 'failed'
}

export interface Attachment {
  url: string
  size: number
  thumbnail: string
  fileName: string
  ext?: string
  isPrivate?: boolean
  status?: AttachmentStatus
}

export interface Content {
  type: string
  text?: string
  placeholder?: string
  thumbnail?: string
  duration?: string
  size?: number
  width?: number
  height?: number
  mentions?: string[]
  mentionAll?: boolean
  reply?: string
  replyContent?: string
  createdAt?: string
  attachment?: Attachment
  extra?: Extra
  encrypted?: boolean
  checksum?: number
  unreadable?: boolean
}

export function newContent(type: ContentType): Content {
  const c: Content = { type: type }
  return c
}

export function newTextContent(text: string): Content {
  const c: Content = { type: ContentType.Text, text: text }
  return c
}

export function parseAttachment(o: Json): Attachment {
  // server may send `path` (upload result) or `url`
  const a: Attachment = {
    url: jstr(o, 'url', jstr(o, 'path')),
    size: jnum(o, 'size'),
    thumbnail: jstr(o, 'thumbnail'),
    fileName: jstr(o, 'fileName'),
    ext: jstr(o, 'ext'),
    isPrivate: jbool(o, 'isPrivate')
  }
  return a
}

export function parseContent(o: Json): Content {
  const c: Content = {
    type: jstr(o, 'type'),
    text: jstr(o, 'text'),
    placeholder: jstr(o, 'placeholder'),
    thumbnail: jstr(o, 'thumbnail'),
    duration: jstr(o, 'duration'),
    size: jnum(o, 'size'),
    width: jnum(o, 'width'),
    height: jnum(o, 'height'),
    mentions: jstrArr(o, 'mentions'),
    mentionAll: jbool(o, 'mentionAll'),
    reply: jstr(o, 'reply'),
    replyContent: jstr(o, 'replyContent'),
    createdAt: jstr(o, 'createdAt'),
    encrypted: jbool(o, 'encrypted'),
    checksum: jnum(o, 'checksum'),
    unreadable: jbool(o, 'unreadable')
  }
  const att = jobj(o, 'attachment')
  if (att !== undefined) {
    c.attachment = parseAttachment(att)
  }
  const extra = jobj(o, 'extra')
  if (extra !== undefined) {
    c.extra = extra
  }
  return c
}
