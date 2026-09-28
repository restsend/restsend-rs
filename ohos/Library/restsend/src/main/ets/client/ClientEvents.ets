import { Conversation } from '../model/Conversation'
import { ChatRequest } from '../model/ChatRequest'
import { MessageStatus } from './Callback'

/**
 * Client event sink. Create once, assign the handlers you need:
 *
 *   client.events.onConnected = (): void => { ... }
 *   client.events.onTopicMessage = (topicId: string, m: ChatRequest): MessageStatus => {
 *     return { hasRead: false, unreadCountable: true }
 *   }
 */
export class ClientEvents {
  onConnected: () => void = (): void => {}
  onConnecting: () => void = (): void => {}
  onBroken: (reason: string) => void = (reason: string): void => {}
  onTokenExpired: (reason: string) => void = (reason: string): void => {}
  onKickoff: (reason: string) => void = (reason: string): void => {}
  onPingFailed: (reason: string) => void = (reason: string): void => {}
  /** return MessageStatus to control read/unread handling of this frame */
  onTopicMessage: (topicId: string, message: ChatRequest) => MessageStatus =
    (topicId: string, message: ChatRequest): MessageStatus => {
      const s: MessageStatus = { hasRead: false, unreadCountable: true }
      return s
    }
  onTopicRead: (topicId: string, message: ChatRequest) => void =
    (topicId: string, message: ChatRequest): void => {}
  onTopicTyping: (topicId: string, message: string) => void =
    (topicId: string, message: string): void => {}
  onConversationsUpdated: (conversations: Conversation[], total: number) => void =
    (conversations: Conversation[], total: number): void => {}
  onConversationRemoved: (topicId: string) => void = (topicId: string): void => {}
  /** convenience: chat logs of one topic changed (new message, ack, recall) */
  onLogsChanged: (topicId: string) => void = (topicId: string): void => {}
  /** server-initiated system frames */
  onSystemRequest: (message: ChatRequest) => void = (message: ChatRequest): void => {}
  /** anything the sdk did not handle */
  onUnknownRequest: (message: ChatRequest) => void = (message: ChatRequest): void => {}
}
