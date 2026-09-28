import { ChatRequest, ReqType } from '../model/ChatRequest'
import { Content, ContentType, newContent, Attachment } from '../model/Content'
import { Extra } from '../model/User'
import { randomChatId } from '../common/Ids'

/** optional rich-send parameters, parity with the rust senders */
export interface SendOptions {
  mentions?: string[]
  mentionAll?: boolean
  replyId?: string
}

function applySendOptions(req: ChatRequest, opts?: SendOptions): ChatRequest {
  if (opts === undefined) {
    return req
  }
  const content: Content = req.content ?? { type: ContentType.Text }
  if (opts.mentions !== undefined && opts.mentions.length > 0) {
    content.mentions = opts.mentions
  }
  if (opts.mentionAll === true) {
    content.mentionAll = true
  }
  if (opts.replyId !== undefined && opts.replyId !== '') {
    content.reply = opts.replyId
  }
  req.content = content
  return req
}

function newChat(topicId: string, type: ContentType): ChatRequest {
  const req: ChatRequest = {
    type: ReqType.Chat,
    chatId: randomChatId(),
    topicId: topicId,
    content: newContent(type)
  }
  return req
}

export function newTextRequest(topicId: string, text: string, opts?: SendOptions): ChatRequest {
  const req = newChat(topicId, ContentType.Text)
  const content: Content = { type: ContentType.Text, text: text }
  req.content = content
  applySendOptions(req, opts)
  return req
}

export function newTypingRequest(topicId: string): ChatRequest {
  const req: ChatRequest = { type: ReqType.Typing, topicId: topicId }
  return req
}

export function newReadRequest(topicId: string, lastReadSeq: number): ChatRequest {
  const req: ChatRequest = { type: ReqType.Read, topicId: topicId, seq: lastReadSeq }
  return req
}

export function newRecallRequest(topicId: string, chatId: string): ChatRequest {
  const req = newChat(topicId, ContentType.Recall)
  req.content = { type: ContentType.Recall, text: chatId } as Content
  return req
}

export function newLocationRequest(topicId: string, lat: string, lng: string, address: string): ChatRequest {
  const req = newChat(topicId, ContentType.Location)
  req.content = { type: ContentType.Location, text: `${lat},${lng}`, placeholder: address } as Content
  return req
}

export function newLinkRequest(topicId: string, url: string, placeholder: string): ChatRequest {
  const req = newChat(topicId, ContentType.Link)
  req.content = { type: ContentType.Link, text: url, placeholder: placeholder } as Content
  return req
}

export function newInviteRequest(topicId: string, message: string): ChatRequest {
  const req = newChat(topicId, ContentType.Invite)
  req.content = { type: ContentType.Invite, text: message } as Content
  return req
}

export function newLogsRequest(topicId: string, attachment: Attachment): ChatRequest {
  const req = newChat(topicId, ContentType.Logs)
  req.content = { type: ContentType.Logs, attachment: attachment } as Content
  return req
}

export function doApplySendOptions(req: ChatRequest, opts?: SendOptions): ChatRequest {
  return applySendOptions(req, opts)
}

export function newUpdateExtraRequest(topicId: string, chatId: string, extra: Extra): ChatRequest {
  const req = newChat(topicId, ContentType.UpdateExtra)
  req.content = { type: ContentType.UpdateExtra, text: chatId, extra: extra } as Content
  return req
}

/** Keepalive frame; content.text is `{"timestamp":...,"error_logs":[...]}`. */
export function newPingRequest(errorLogs: string[]): ChatRequest {
  const payload = JSON.stringify({ timestamp: Date.now(), error_logs: errorLogs })
  const req: ChatRequest = {
    type: ReqType.Ping,
    chatId: randomChatId(),
    content: { type: 'ping', text: payload } as Content
  }
  return req
}
