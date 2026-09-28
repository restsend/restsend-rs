// Models and protocol of the RestSend client (ArkTS port of crates/restsend).
// ---- common ----
export { Json, parseJson, jstr, jnum, jbool, jobj, jarr, jstrArr } from './common/Json'
export { randomChatId, randomNonce, nowMillis } from './common/Ids'

// ---- models ----
export { AuthInfo, User, Extra, authInfo, parseAuthInfo, parseUser } from './model/User'
export {
  ContentType, AttachmentStatus, Attachment, Content,
  newContent, newTextContent, parseAttachment, parseContent
} from './model/Content'
export { ReqType, ChatRequest, parseChatRequest, parseChatRequestText } from './model/ChatRequest'
export { ChatLogStatus, ChatLog, chatLogFromRequest } from './model/ChatLog'
export { Tag, Conversation, parseConversation, conversationToJson } from './model/Conversation'
export { Topic, parseTopic } from './model/Topic'
export { TopicKnock, parseTopicKnocks, parseTopicKnock,
  GetChatLogsResult, ApiSendResponse, parseApiSendResponse } from './model/TopicKnock'
export { Upload, parseUpload } from './model/Upload'

// ---- protocol ----
export {
  newTextRequest, newTypingRequest, newReadRequest, newRecallRequest,
  newLocationRequest, newLinkRequest, newInviteRequest, newLogsRequest,
  newUpdateExtraRequest, newPingRequest, SendOptions, doApplySendOptions
} from './protocol/Requests'

// ---- transports / storage abstraction ----
export {
  HttpResponse, HttpApi, WsEvents, WsTransport, SDK_VERSION, connectUrl
} from './net/Transport'
export { Row, Table, MemoryTable } from './store/Table'

// ---- rest api ----
export { Api, ApiError, ChatListResult, ChatSyncResult, initHttpAdapter } from './api/Api'

// ---- client ----
export { ClientOptions } from './client/ClientOptions'
export { SendCallback, DownloadCallback, MessageStatus, SyncResult, SyncChatLogsResult } from './client/Callback'
export { ClientEvents } from './client/ClientEvents'
export { ConnState } from './client/ConnectionManager'
export { RestsendClient } from './client/RestsendClient'
export { ClientError, ClientErrorKind } from './common/ClientError'

// ---- @ohos adapters (device side) ----
export { OhosHttp, OhosWs, OhosMedia } from './net/OhosAdapter'
export { RdbTable } from './store/RdbTable'
export { createRestsendClient, OhosAccounts } from './Ohos'
export { MediaApi, UploadRequest, UploadContentRequest } from './net/MediaApi'
export { AttachmentManager } from './client/AttachmentManager'
