/* tslint:disable */
/* eslint-disable */
export function setLogging(level?: string): void;
/**
 * Signin with userId and password or token
 */
export function signin(endpoint: string, userId: string, password?: string, token?: string): Promise<any>;
/**
 * Signup with userId and password
 */
export function signup(endpoint: string, userId: string, password: string): Promise<any>;
/**
 * Logout with token
 */
export function logout(endpoint: string, token: string): Promise<void>;
export class Client {
  free(): void;
  /**
   * Create a new chat with userId
   * return: Conversation    
   */
  createChat(userId: string): Promise<any>;
  /**
   * Clean history of a conversation
   */
  cleanMessages(topicId: string): Promise<void>;
  /**
   * Remove messages from a conversation
   */
  removeMessages(topicId: string, chatIds: (string)[]): Promise<void>;
  /**
   * Sync chat logs from server
   * #Arguments
   * * `topicId` - topic id
   * * `lastSeq` - Number, last seq
   * * `option` - option
   *     * `limit` - limit
   *     * `ensureConversationVersion` - ensure conversation version, default false
   *     * `onsuccess` - onsuccess callback -> function (result: GetChatLogsResult)
   *     * `onerror` - onerror callback -> function (error: String)
   */
  syncChatLogs(topicId: string, lastSeq: number | undefined, option: any): Promise<void>;
  saveChatLogs(logs: any): Promise<void>;
  /**
   * Sync conversations from server
   * #Arguments
   * * `option` - option
   *    * `syncLogs` - syncs logs, default false
   *    * `syncLogsLimit` - sync logs limit, per conversation, default 100
   *    * `syncLogsMaxCount` - sync logs max count, default 200
   *    * `limit` - limit
   *    * `updatedAt` String - updated_at optional
   *    * `lastRemovedAt` String - last_removed_at optional
   *    * `onsuccess` - onsuccess callback -> function (updated_at:String, count: u32)
   *         - updated_at: last updated_at
   *         - count: count of conversations, if count == limit, there may be more conversations, you can call syncConversations again with updated_at, stop when count < limit
   *    * `onerror` - onerror callback -> function (error: String)
   */
  syncConversations(option: any): Promise<void>;
  /**
   * Get conversation by topicId
   * #Arguments
   * * `topicId` - topic id
   * * `blocking` - blocking optional
   * return: Conversation or null
   */
  getConversation(topicId: string, blocking?: boolean): Promise<any>;
  /**
   * Remove conversation by topicId
   * #Arguments
   * * `topicId` - topic id
   */
  removeConversation(topicId: string): Promise<void>;
  /**
   * Set conversation remark
   * #Arguments
   * * `topicId` - topic id
   * * `remark` - remark
   */
  setConversationRemark(topicId: string, remark?: string): Promise<any>;
  /**
   * Set conversation sticky by topicId
   * #Arguments
   * * `topicId` - topic id
   * * `sticky` - sticky
   */
  setConversationSticky(topicId: string, sticky: boolean): Promise<any>;
  /**
   * Set conversation mute by topicId
   * #Arguments
   * * `topicId` - topic id
   * * `mute` - mute
   */
  setConversationMute(topicId: string, mute: boolean): Promise<any>;
  /**
   * Set conversation read by topicId
   * #Arguments
   * * `topicId` - topic id
   * * `heavy` - heavy optional
   */
  setConversationRead(topicId: string, heavy?: boolean): Promise<void>;
  /**
   * Set conversation read by topicId
   * #Arguments
   * * `topicId` - topic id
   * * `heavy` - heavy optional
   */
  setAllConversationsRead(): Promise<void>;
  /**
   * Set conversation tags
   * #Arguments
   * * `topicId` - topic id
   * * `tags` - tags is array of Tag:
   *     - id - string
   *     - type - string
   *     - label - string
   */
  setConversationTags(topicId: string, tags: any): Promise<any>;
  /**
   * Clear conversation on local storage
   * #Arguments
   * * `topicId` - topic id
   */
  clearConversation(topicId: string): Promise<void>;
  /**
   * Set conversation extra
   * #Arguments
   * * `topicId` - topic id
   * # `extra` - extra
   * # Return: Conversation
   */
  setConversationExtra(topicId: string, extra: any): Promise<any>;
  /**
   * Filter conversation with options
   * #Arguments
   * * `predicate` - filter predicate
   *     -> return true to keep the conversation
   * * `lastUpdatedAt` - last updated_at
   * * `limit` - limit
   * #Return Array of Conversation
   * #Example
   * ```js
   * const conversations = client.filterConversation((c) => {
   *    return c.remark === 'hello'
   * })
   * ```
   * #Example
   * ```js
   * const conversations = await client.filterConversation((c) => {
   *   return c.remark === 'hello' && c.tags && c.tags.some(t => t.label === 'hello')
   * })
   */
  filterConversation(predicate: any, lastUpdatedAt: any, limit: any): Promise<any>;
  /**
   * Get user info
   * #Arguments
   * * `userId` - user id
   * * `blocking` - blocking fetch from server
   * #Return
   * User info
   */
  getUser(userId: string, blocking?: boolean): Promise<any>;
  /**
   * Get multiple users info
   * #Arguments
   * * `userIds` - Array of user id
   * #Return
   * Array of user info
   */
  getUsers(userIds: (string)[]): Promise<any>;
  /**
   * Set user remark name
   * #Arguments
   * * `userId` - user id
   * * `remark` - remark name
   */
  setUserRemark(userId: string, remark: string): Promise<void>;
  /**
   * Set user star
   * #Arguments
   * * `userId` - user id
   * * `star` - star
   */
  setUserStar(userId: string, star: boolean): Promise<void>;
  /**
   * Set user block
   * #Arguments
   * * `userId` - user id
   * * `block` - block
   */
  setUserBlock(userId: string, block: boolean): Promise<void>;
  /**
   * Set allow guest chat
   * #Arguments
   * * `allow` - allow
   */
  setAllowGuestChat(allow: boolean): Promise<void>;
  /**
   * Create a new topic
   * #Arguments
   *   name: String,
   *  icon: String,
   * #Return
   * * `Topic` || `undefined`
   */
  createTopic(members: (string)[], name?: string, icon?: string): Promise<any>;
  /**
   * Join a topic
   * #Arguments
   * * `topicId` - topic id
   * * `message` - message
   * * `source` - source
   */
  joinTopic(topicId: string, message?: string, source?: string): Promise<void>;
  /**
   * Add user into topic
   * #Arguments
   * * `topicId` - topic id
   * * `userId` - user id
   * #Return
   * * `TopicMember` || `undefined`
   */
  addMember(topicId: string, userId: string): Promise<any>;
  /**
   * Get topic info
   * #Arguments
   * * `topicId` - topic id
   * #Return
   * * `Topic` || `undefined`
   */
  getTopic(topicId: string): Promise<any>;
  /**
   * Get topic admins
   * #Arguments
   * * `topicId` - topic id
   * #Return
   * * `Vec<User>` || `undefined`
   */
  getTopicAdmins(topicId: string): Promise<any>;
  /**
   * Get topic owner
   * #Arguments
   * * `topicId` - topic id
   * #Return
   * * `User` || `undefined`
   */
  getTopicOwner(topicId: string): Promise<any>;
  /**
   * Get topic members
   * #Arguments
   * * `topicId` - topic id
   * * `updatedAt` - updated_at
   * * `limit` - limit
   * #Return
   * * `ListUserResult` || `undefined`
   */
  getTopicMembers(topicId: string, updatedAt: string, limit: number): Promise<any>;
  /**
   * Get topic knocks
   * #Arguments
   * * `topicId` - topic id
   * #Return
   * * `Vec<TopicKnock>`
   */
  getTopicKnocks(topicId: string): Promise<any>;
  /**
   * Update topic info
   * #Arguments
   * * `topicId` - topic id
   * * `option` - option
   *     * `name` - String
   *     * `icon` - String (url) or base64
   */
  updateTopic(topicId: string, option: any): Promise<void>;
  /**
   * Update topic notice
   * #Arguments
   * * `topicId` - topic id
   * * `text` - notice text
   */
  updateTopicNotice(topicId: string, text: string): Promise<void>;
  /**
   * Silence topic
   * #Arguments
   * * `topicId` - topic id
   * * `duration` - duration, format: 1d, 1h, 1m, cancel with empty string
   */
  silentTopic(topicId: string, duration?: string): Promise<void>;
  /**
   * Silent topic member
   * #Arguments
   * * `topicId` - topic id
   * * `userId` - user id
   * * `duration` - duration, format: 1d, 1h, 1m, cancel with empty string
   */
  silentTopicMember(topicId: string, userId: string, duration?: string): Promise<void>;
  /**
   * Add topic admin
   * #Arguments
   * * `topicId` - topic id
   * * `userId` - user id
   */
  addTopicAdmin(topicId: string, userId: string): Promise<void>;
  /**
   * Remove topic admin
   * #Arguments
   * * `topicId` - topic id
   * * `userId` - user id
   */
  removeTopicAdmin(topicId: string, userId: string): Promise<void>;
  /**
   * Transfer topic
   * #Arguments
   * * `topicId` - topic id
   * * `userId` - user id to transfer, the user must be a topic member
   */
  transferTopic(topicId: string, userId: string): Promise<void>;
  /**
   * Quit topic
   * #Arguments
   * * `topicId` - topic id
   */
  quitTopic(topicId: string): Promise<void>;
  /**
   * Dismiss topic
   * #Arguments
   * * `topicId` - topic id
   */
  dismissTopic(topicId: string): Promise<void>;
  /**
   * Accept topic join
   * #Arguments
   * * `topicId` - topic id
   * * `userId` - user id
   * * `memo` - accept memo
   */
  acceptTopicJoin(topicId: string, userId: string, memo?: string): Promise<void>;
  /**
   * Decline topic join
   * #Arguments
   * * `topicId` - topic id
   * * `userId` - user id
   * * `message` - decline message
   */
  declineTopicJoin(topicId: string, userId: string, message?: string): Promise<void>;
  /**
   * Remove topic member
   * #Arguments
   * * `topicId` - topic id
   * * `userId` - user id
   */
  removeTopicMember(topicId: string, userId: string): Promise<void>;
  /**
   * Create a new client
   * # Arguments
   * * `info` - AuthInfo
   * * `db_name` - database name (optional), create an indexeddb when set it    
   */
  constructor(info: any, db_name?: string);
  /**
   * connect immediately if the connection is broken    
   */
  app_active(): void;
  shutdown(): Promise<void>;
  connect(): Promise<void>;
  /**
   *
   * Send message with content
   * # Arguments
   * * `topicId` - The topic id
   * * `content` - The content Object
   *     * `type` String - The content type, must be [text, image, video, audio, file, YOUR_CUSTOM_TYPE]
   *     * `text` String - The text message
   *     * `attachment` Object - The attachment object
   *     * `duration` String - The duration of the content, only for video and audio, optional, format is hh:mm:ss
   *     * `thumbnail` Object - The thumbnail object, only for video and image, optional
   *     * `size` Number - The size of the content, only for file, optional
   *     * `placeholder` String - The placeholder of the content, optional
   *     * `width` Number - The width of the content, only for image/video, optional
   *     * `height` Number - The height of the content, only for image/video, optional
   *     * `reply` String - The reply message id, optional
   *     * `mentions` Array - Mention to users, optional
   *     * `mentionsAll` Boolean - Mention to all users, optional
   * * `option` - The send option
   * # Return
   * The message id
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * await client.doSend(topicId, {
   *     type: 'wx.text',
   *     text: 'hello',
   * }, {
   *     mentions: undefined, // The mention user id list, optional
   *     mentionAll:  false, // Mention all users, optional
   *     reply:  undefined, // The reply message id, optional
   *     onsent:  () => {}, // The callback when message sent
   *     onprogress:  (progress:Number, total:Number)  =>{}, // The callback when message sending progress
   *     onattachmentupload:  (result:Upload) => { }, // The callback when attachment uploaded, return the Content object to replace the original content
   *     onack:  (req:ChatRequest)  => {}, // The callback when message acked
   *     onfail:  (reason:String)  => {} // The callback when message failed
   * });
   * ```
   */
  doSend(topicId: string, content: any, option: any): Promise<string>;
  /**
   * Send typing status
   * # Arguments
   * * `topicId` - The topic id    
   */
  doTyping(topicId: string): Promise<void>;
  /**
   * Recall message
   * # Arguments
   * * `topicId` - The topic id
   * * `messageId` - The message id
   */
  doRecall(topicId: string, messageId: string, option: any): Promise<string>;
  /**
   * Send voice message
   * # Arguments
   * * `topicId` - The topic id
   * * `attachment` - The attachment object
   * * `option` - The send option
   *     * `duration` String - The duration of the content, only for video and audio, optional, format is hh:mm:ss
   *     * `mentions` Array - The mention user id list, optional
   *     * `mentionAll` boolean, // Mention all users, optional
   *     * `reply` String - The reply message id, optional
   * # Return
   * The message id
   */
  doSendVoice(topicId: string, attachment: any, option: any): Promise<string>;
  /**
   * Send video message
   * # Arguments
   * * `topicId` - The topic id
   * * `attachment` - The attachment object
   * * `option` - The send option
   *    * `duration` String - The duration of the content, only for video and audio, optional, format is hh:mm:ss
   *    * `mentions` Array - The mention user id list, optional
   *    * `mentionAll` boolean, // Mention all users, optional
   *    * `reply` String - The reply message id, optional
   * # Return
   * The message id
   */
  doSendVideo(topicId: string, attachment: any, option: any): Promise<string>;
  /**
   * Send file message
   * # Arguments
   * * `topicId` - The topic id
   * * `attachment` - The attachment object
   * * `option` - The send option
   *    * `size` Number - The size of the content, only for file, optional
   *    * `mentions` Array - The mention user id list, optional
   *    * `mentionAll` boolean, // Mention all users, optional
   *    * `reply` String - The reply message id, optional
   * # Return
   * The message id
   */
  doSendFile(topicId: string, attachment: any, option: any): Promise<string>;
  /**
   * Send location message
   * # Arguments
   * * `topicId` - The topic id
   * * `latitude` - The latitude
   * * `longitude` - The longitude
   * * `address` - The address
   * * `option` - The send option
   *   * `mentions` Array - The mention user id list, optional
   *   * `mentionAll` boolean, // Mention all users, optional
   *   * `reply` String - The reply message id, optional
   * # Return
   * The message id
   */
  doSendLocation(topicId: string, latitude: string, longitude: string, address: string, option: any): Promise<string>;
  /**
   * Send link message
   * # Arguments
   * * `topicId` - The topic id
   * * `url` - The url
   * * `option` - The send option
   *  * `placeholder` String - The placeholder of the content, optional
   *  * `mentions` Array - The mention user id list, optional
   *  * `mentionAll` boolean, // Mention all users, optional
   *  * `reply` String - The reply message id, optional
   * # Return
   * The message id
   */
  doSendLink(topicId: string, url: string, option: any): Promise<string>;
  /**
   * Send invite message
   * # Arguments
   * * `topicId` - The topic id
   * * `logIds` Array - The log id list
   * * `option` - The send option
   * # Return    
   * The message id
   */
  doSendLogs(topicId: string, sourceTopicId: string, logIds: (string)[], option: any): Promise<string>;
  /**
   * Send text message
   * # Arguments
   * * `topicId` - The topic id
   * * `text` - The text message
   * * `option` - The send option
   * # Return
   * The message id
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * await client.sendText(topicId, text, {
   *     mentions: [] || undefined, // The mention user id list, optional
   *     reply: String || undefined, - The reply message id, optional
   *     onsent:  () => {},
   *     onprogress:  (progress:Number, total:Number)  =>{},
   *     onack:  (req:ChatRequest)  => {},
   *     onfail:  (reason:String)  => {}
   * });
   * ```
   */
  doSendText(topicId: string, text: string, option: any): Promise<string>;
  /**
   *
   * Send image message
   * # Arguments
   * * `topicId` - The topic id
   * * `attachment` - The attachment object
   *     * `file` File - The file object
   *     * `url` String  - The file name
   * * `option` - The send option
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * await client.sendImage(topicId, {file:new File(['(⌐□_□)'], 'hello_restsend.png', { type: 'image/png' })}, {});
   * ```
   */
  doSendImage(topicId: string, attachment: any, option: any): Promise<string>;
  /**
   * Update sent chat message's extra
   * # Arguments
   * * `topicId` - The topic id
   * * `chatId` - The chat id
   * * `extra` - The extra, optional
   * * `option` - The send option
   * # Return
   * The message id
   */
  doUpdateExtra(topicId: string, chatId: string, extra: any, option: any): Promise<string>;
  /**
   * Set the callback when connection connected
   */
  set onconnected(value: any);
  /**
   * Set the callback when connection connecting
   */
  set onconnecting(value: any);
  /**
   * Set the callback when connection token expired
   */
  set ontokenexpired(value: any);
  /**
   * Set the callback when connection broken
   * # Arguments
   * * `reason` String - The reason of the connection broken
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * client.onnetbroken = (reason) => {
   * console.log(reason);
   * }
   * ```
   */
  set onbroken(value: any);
  /**
   * Set the callback when kickoff by other client
   * # Arguments
   * * `reason` String - The reason of the kickoff
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * client.onkickoff = (reason) => {
   * console.log(reason);
   * }
   * ```
   */
  set onkickoff(value: any);
  /**
   * Set the callback when receive system request
   * # Arguments
   *  * `req` - The request object, the return value is the response object
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * client.onsystemrequest = (req) => {
   *    if (req.type === 'get') {
   *       return {type:'resp', code: 200}
   *   }
   * }
   * ```
   */
  set onsystemrequest(value: any);
  /**
   * Set the callback when receive unknown request
   * # Arguments
   *  * `req` - The request object, the return value is the response object
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * client.onunknownrequest = (req) => {
   *   if (req.type === 'get') {
   *      return {type:'resp', code: 200}
   *  }
   * }
   */
  set onunknownrequest(value: any);
  /**
   * Set the callback when receive typing event
   * # Arguments
   * * `topicId` String - The topic id
   * * `message` ChatRequest - The message
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * client.ontyping = (topicId, message) => {
   *  console.log(topicId, message);
   * }
   * ```
   */
  set ontopictyping(value: any);
  /**
   * Set the callback when receive new message
   * # Arguments
   * * `topicId` String - The topic id
   * * `message` ChatRequest - The message
   * # Return
   * * `true` - If return true, will send `has read` to server
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * client.ontopicmessage = (topicId, message) => {
   * console.log(topicId, message);
   * return true;
   * }
   * ```
   */
  set ontopicmessage(value: any);
  /**
   * Set the callback when receive read event
   * # Arguments
   * * `topicId` String - The topic id
   * * `message` ChatRequest - The message
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * client.ontopicread = (topicId, message) => {
   * console.log(topicId, message);
   * }
   * ```
   */
  set ontopicread(value: any);
  /**
   * Set the callback when conversations updated
   * # Arguments
   * * `conversations` - The conversation list
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * client.onconversationsupdated = (conversations) => {
   * console.log(conversations);
   * }
   * ```
   */
  set onconversationsupdated(value: any);
  /**
   * Set the callback when conversations removed
   * # Arguments
   * * `conversationId` - The conversation id
   * # Example
   * ```javascript
   * const client = new Client(info);
   * await client.connect();
   * client.onconversationsremoved = (conversationId) => {
   * console.log(conversationId);
   * }
   * ```
   */
  set onconversationsremoved(value: any);
  /**
   * get the current connection status
   * return: connecting, connected, broken, shutdown
   */
  readonly connectionStatus: string;
  /**
   * set the keepalive interval with seconds
   */
  set keepalive(value: number);
}
export class IntoUnderlyingByteSource {
  private constructor();
  free(): void;
  start(controller: ReadableByteStreamController): void;
  pull(controller: ReadableByteStreamController): Promise<any>;
  cancel(): void;
  readonly type: string;
  readonly autoAllocateChunkSize: number;
}
export class IntoUnderlyingSink {
  private constructor();
  free(): void;
  write(chunk: any): Promise<any>;
  close(): Promise<any>;
  abort(reason: any): Promise<any>;
}
export class IntoUnderlyingSource {
  private constructor();
  free(): void;
  pull(controller: ReadableStreamDefaultController): Promise<any>;
  cancel(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly client_set_onconnected: (a: number, b: number) => void;
  readonly client_set_onconnecting: (a: number, b: number) => void;
  readonly client_set_ontokenexpired: (a: number, b: number) => void;
  readonly client_set_onbroken: (a: number, b: number) => void;
  readonly client_set_onkickoff: (a: number, b: number) => void;
  readonly client_set_onsystemrequest: (a: number, b: number) => void;
  readonly client_set_onunknownrequest: (a: number, b: number) => void;
  readonly client_set_ontopictyping: (a: number, b: number) => void;
  readonly client_set_ontopicmessage: (a: number, b: number) => void;
  readonly client_set_ontopicread: (a: number, b: number) => void;
  readonly client_set_onconversationsupdated: (a: number, b: number) => void;
  readonly client_set_onconversationsremoved: (a: number, b: number) => void;
  readonly setLogging: (a: number, b: number) => void;
  readonly client_createChat: (a: number, b: number, c: number) => number;
  readonly client_cleanMessages: (a: number, b: number, c: number) => number;
  readonly client_removeMessages: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_syncChatLogs: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
  readonly client_saveChatLogs: (a: number, b: number) => number;
  readonly client_syncConversations: (a: number, b: number) => number;
  readonly client_getConversation: (a: number, b: number, c: number, d: number) => number;
  readonly client_removeConversation: (a: number, b: number, c: number) => number;
  readonly client_setConversationRemark: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_setConversationSticky: (a: number, b: number, c: number, d: number) => number;
  readonly client_setConversationMute: (a: number, b: number, c: number, d: number) => number;
  readonly client_setConversationRead: (a: number, b: number, c: number, d: number) => number;
  readonly client_setAllConversationsRead: (a: number) => number;
  readonly client_setConversationTags: (a: number, b: number, c: number, d: number) => number;
  readonly client_clearConversation: (a: number, b: number, c: number) => number;
  readonly client_setConversationExtra: (a: number, b: number, c: number, d: number) => number;
  readonly client_filterConversation: (a: number, b: number, c: number, d: number) => number;
  readonly client_getUser: (a: number, b: number, c: number, d: number) => number;
  readonly client_getUsers: (a: number, b: number, c: number) => number;
  readonly client_setUserRemark: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_setUserStar: (a: number, b: number, c: number, d: number) => number;
  readonly client_setUserBlock: (a: number, b: number, c: number, d: number) => number;
  readonly client_setAllowGuestChat: (a: number, b: number) => number;
  readonly client_createTopic: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
  readonly client_joinTopic: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
  readonly client_addMember: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_getTopic: (a: number, b: number, c: number) => number;
  readonly client_getTopicAdmins: (a: number, b: number, c: number) => number;
  readonly client_getTopicOwner: (a: number, b: number, c: number) => number;
  readonly client_getTopicMembers: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
  readonly client_getTopicKnocks: (a: number, b: number, c: number) => number;
  readonly client_updateTopic: (a: number, b: number, c: number, d: number) => number;
  readonly client_updateTopicNotice: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_silentTopic: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_silentTopicMember: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
  readonly client_addTopicAdmin: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_removeTopicAdmin: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_transferTopic: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_quitTopic: (a: number, b: number, c: number) => number;
  readonly client_dismissTopic: (a: number, b: number, c: number) => number;
  readonly client_acceptTopicJoin: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
  readonly client_declineTopicJoin: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
  readonly client_removeTopicMember: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly signin: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
  readonly signup: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
  readonly logout: (a: number, b: number, c: number, d: number) => number;
  readonly __wbg_client_free: (a: number, b: number) => void;
  readonly client_new: (a: number, b: number, c: number) => number;
  readonly client_connectionStatus: (a: number, b: number) => void;
  readonly client_app_active: (a: number) => void;
  readonly client_set_keepalive: (a: number, b: number) => void;
  readonly client_shutdown: (a: number) => number;
  readonly client_connect: (a: number) => number;
  readonly client_doSend: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_doTyping: (a: number, b: number, c: number) => number;
  readonly client_doRecall: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
  readonly client_doSendVoice: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_doSendVideo: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_doSendFile: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_doSendLocation: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => number;
  readonly client_doSendLink: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
  readonly client_doSendLogs: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
  readonly client_doSendText: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
  readonly client_doSendImage: (a: number, b: number, c: number, d: number, e: number) => number;
  readonly client_doUpdateExtra: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => number;
  readonly __wbg_intounderlyingbytesource_free: (a: number, b: number) => void;
  readonly intounderlyingbytesource_type: (a: number, b: number) => void;
  readonly intounderlyingbytesource_autoAllocateChunkSize: (a: number) => number;
  readonly intounderlyingbytesource_start: (a: number, b: number) => void;
  readonly intounderlyingbytesource_pull: (a: number, b: number) => number;
  readonly intounderlyingbytesource_cancel: (a: number) => void;
  readonly __wbg_intounderlyingsink_free: (a: number, b: number) => void;
  readonly intounderlyingsink_write: (a: number, b: number) => number;
  readonly intounderlyingsink_close: (a: number) => number;
  readonly intounderlyingsink_abort: (a: number, b: number) => number;
  readonly __wbg_intounderlyingsource_free: (a: number, b: number) => void;
  readonly intounderlyingsource_pull: (a: number, b: number) => number;
  readonly intounderlyingsource_cancel: (a: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __wbindgen_export_3: WebAssembly.Table;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h6bd4ca820affe721: (a: number, b: number, c: number) => void;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h8a4db968a8f99d12: (a: number, b: number) => void;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h66556920b1f35d36: (a: number, b: number, c: number) => void;
  readonly wasm_bindgen__convert__closures__invoke2_mut__h3767371f6ec92a1e: (a: number, b: number, c: number, d: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
