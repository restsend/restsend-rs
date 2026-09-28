import { ChatRequest } from '../model/ChatRequest'

/** Per-message send callbacks. Assign the fields you care about. */
export class SendCallback {
  onSent: (req: ChatRequest) => void = (req: ChatRequest): void => {}
  onAck: (req: ChatRequest) => void = (req: ChatRequest): void => {}
  onFail: (reason: string) => void = (reason: string): void => {}
  /** upload progress for attachment sends */
  onProgress: (sent: number, total: number) => void = (sent: number, total: number): void => {}
}

/** download_file callbacks, parity with rust DownloadCallback */
export class DownloadCallback {
  onProgress: (sent: number, total: number) => void = (sent: number, total: number): void => {}
  /** path of the written file */
  onSuccess: (path: string) => void = (path: string): void => {}
  onFail: (reason: string) => void = (reason: string): void => {}
}

/** Returned by ClientEvents.onTopicMessage. */
export interface MessageStatus {
  hasRead: boolean
  unreadCountable: boolean
}

/** Async result of syncConversations. */
export interface SyncResult {
  updatedAt: string
  count: number
  total: number
}

export interface SyncChatLogsResult {
  topicId: string
  hasMore: boolean
  lastSeq: number
  count: number
}
