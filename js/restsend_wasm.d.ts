/* tslint:disable */
/* eslint-disable */
/**
* @param {string | undefined} [level]
*/
export function setLogging(level?: string): void;
/**
* Signin with userId and password or token
* @param {string} endpoint
* @param {string} userId
* @param {string | undefined} [password]
* @param {string | undefined} [token]
* @returns {Promise<any>}
*/
export function signin(endpoint: string, userId: string, password?: string, token?: string): Promise<any>;
/**
* Signup with userId and password
* @param {string} endpoint
* @param {string} userId
* @param {string} password
* @returns {Promise<any>}
*/
export function signup(endpoint: string, userId: string, password: string): Promise<any>;
/**
* Logout with token
* @param {string} endpoint
* @param {string} token
* @returns {Promise<void>}
*/
export function logout(endpoint: string, token: string): Promise<void>;
/**
*/
export class Client {
  free(): void;
/**
* @param {any} info
*/
  constructor(info: any);
/**
* @returns {Promise<void>}
*/
  shutdown(): Promise<void>;
/**
* @returns {Promise<void>}
*/
  connect(): Promise<void>;
/**
* Create a new chat with userId
* return: Conversation
* @param {string} userId
* @returns {Promise<any>}
*/
  createChat(userId: string): Promise<any>;
/**
* Clean history of a conversation
* @param {string} topicId
* @returns {Promise<void>}
*/
  cleanMessages(topicId: string): Promise<void>;
/**
* Remove messages from a conversation
* @param {string} topicId
* @param {(string)[]} chatIds
* @returns {Promise<void>}
*/
  removeMessages(topicId: string, chatIds: (string)[]): Promise<void>;
/**
* Sync chat logs from server
* #Arguments
* * `topicId` - topic id
* * `lastSeq` - Number, last seq
* * `option` - option
*     * `limit` - limit
*     * `onsuccess` - onsuccess callback -> function (result: GetChatLogsResult)
*     * `onerror` - onerror callback -> function (error: String)
* @param {string} topicId
* @param {number | undefined} lastSeq
* @param {any} option
* @returns {Promise<void>}
*/
  syncChatLogs(topicId: string, lastSeq: number | undefined, option: any): Promise<void>;
/**
* Sync conversations from server
* #Arguments
* * `option` - option
*    * `limit` - limit
*    * `updatedAt` String - updated_at optional
*    * `onsuccess` - onsuccess callback -> function (updated_at:String, count: u32)
*         - updated_at: last updated_at
*         - count: count of conversations, if count == limit, there may be more conversations, you can call syncConversations again with updated_at, stop when count < limit
*    * `onerror` - onerror callback -> function (error: String)
* @param {any} option
* @returns {Promise<void>}
*/
  syncConversations(option: any): Promise<void>;
/**
* Get conversation by topicId
* #Arguments
* * `topicId` - topic id
* return: Conversation or null
* @param {string} topicId
* @returns {any}
*/
  getConversation(topicId: string): any;
/**
* Remove conversation by topicId
* #Arguments
* * `topicId` - topic id
* @param {string} topicId
* @returns {Promise<void>}
*/
  removeConversation(topicId: string): Promise<void>;
/**
* Set conversation remark
* #Arguments
* * `topicId` - topic id
* * `remark` - remark
* @param {string} topicId
* @param {string | undefined} [remark]
* @returns {Promise<any>}
*/
  setConversationRemark(topicId: string, remark?: string): Promise<any>;
/**
* Set conversation sticky by topicId
* #Arguments
* * `topicId` - topic id
* * `sticky` - sticky
* @param {string} topicId
* @param {boolean} sticky
* @returns {Promise<any>}
*/
  setConversationSticky(topicId: string, sticky: boolean): Promise<any>;
/**
* Set conversation mute by topicId
* #Arguments
* * `topicId` - topic id
* * `mute` - mute
* @param {string} topicId
* @param {boolean} mute
* @returns {Promise<any>}
*/
  setConversationMute(topicId: string, mute: boolean): Promise<any>;
/**
* Set conversation read by topicId
* #Arguments
* * `topicId` - topic id
* @param {string} topicId
* @returns {Promise<void>}
*/
  setConversationRead(topicId: string): Promise<void>;
/**
* Set conversation tags
* #Arguments
* * `topicId` - topic id
* * `tags` - tags is array of Tag:
*     - id - string
*     - type - string
*     - label - string
* @param {string} topicId
* @param {any} tags
* @returns {Promise<any>}
*/
  setConversationTags(topicId: string, tags: any): Promise<any>;
/**
* Set conversation extra
* #Arguments
* * `topicId` - topic id
* # `extra` - extra
* # Return: Conversation
* @param {string} topicId
* @param {any} extra
* @returns {Promise<any>}
*/
  setConversationExtra(topicId: string, extra: any): Promise<any>;
/**
* Filter conversation with options
* #Arguments
* * `predicate` - filter predicate
*     -> return true to keep the conversation
* #Return Array of Conversation
* #Example
* ```js
* const conversations = client.filterConversation((c) => {
*    return c.remark === 'hello'
* })
* ```
* #Example
* ```js
* const conversations = client.filterConversation((c) => {
*   return c.remark === 'hello' && c.tags && c.tags.some(t => t.label === 'hello')
* })
* @param {any} predicate
* @returns {Promise<any>}
*/
  filterConversation(predicate: any): Promise<any>;
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
* @param {string} topicId
* @param {any} content
* @param {any} option
* @returns {Promise<string>}
*/
  doSend(topicId: string, content: any, option: any): Promise<string>;
/**
* Send typing status
* # Arguments
* * `topicId` - The topic id    
* @param {string} topicId
* @returns {Promise<void>}
*/
  doTyping(topicId: string): Promise<void>;
/**
* Recall message
* # Arguments
* * `topicId` - The topic id
* * `messageId` - The message id
* @param {string} topicId
* @param {string} messageId
* @param {any} option
* @returns {Promise<string>}
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
* @param {string} topicId
* @param {any} attachment
* @param {any} option
* @returns {Promise<string>}
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
* @param {string} topicId
* @param {any} attachment
* @param {any} option
* @returns {Promise<string>}
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
* @param {string} topicId
* @param {any} attachment
* @param {any} option
* @returns {Promise<string>}
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
* @param {string} topicId
* @param {string} latitude
* @param {string} longitude
* @param {string} address
* @param {any} option
* @returns {Promise<string>}
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
* @param {string} topicId
* @param {string} url
* @param {any} option
* @returns {Promise<string>}
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
* @param {string} topicId
* @param {string} sourceTopicId
* @param {(string)[]} logIds
* @param {any} option
* @returns {Promise<string>}
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
* @param {string} topicId
* @param {string} text
* @param {any} option
* @returns {Promise<string>}
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
* @param {string} topicId
* @param {any} attachment
* @param {any} option
* @returns {Promise<string>}
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
* @param {string} topicId
* @param {string} chatId
* @param {any} extra
* @param {any} option
* @returns {Promise<string>}
*/
  doUpdateExtra(topicId: string, chatId: string, extra: any, option: any): Promise<string>;
/**
* Get user info
* #Arguments
* * `userId` - user id
* * `blocking` - blocking fetch from server
* #Return
* User info
* @param {string} userId
* @param {boolean | undefined} [blocking]
* @returns {Promise<any>}
*/
  getUser(userId: string, blocking?: boolean): Promise<any>;
/**
* Get multiple users info
* #Arguments
* * `userIds` - Array of user id
* #Return
* Array of user info
* @param {(string)[]} userIds
* @returns {Promise<any>}
*/
  getUsers(userIds: (string)[]): Promise<any>;
/**
* Set user remark name
* #Arguments
* * `userId` - user id
* * `remark` - remark name
* @param {string} userId
* @param {string} remark
* @returns {Promise<void>}
*/
  setUserRemark(userId: string, remark: string): Promise<void>;
/**
* Set user star
* #Arguments
* * `userId` - user id
* * `star` - star
* @param {string} userId
* @param {boolean} star
* @returns {Promise<void>}
*/
  setUserStar(userId: string, star: boolean): Promise<void>;
/**
* Set user block
* #Arguments
* * `userId` - user id
* * `block` - block
* @param {string} userId
* @param {boolean} block
* @returns {Promise<void>}
*/
  setUserBlock(userId: string, block: boolean): Promise<void>;
/**
* Set allow guest chat
* #Arguments
* * `allow` - allow
* @param {boolean} allow
* @returns {Promise<void>}
*/
  setAllowGuestChat(allow: boolean): Promise<void>;
/**
* Create a new topic
* #Arguments
*   name: String,
*  icon: String,
* #Return
* * `Topic` || `undefined`
* @param {(string)[]} members
* @param {string | undefined} [name]
* @param {string | undefined} [icon]
* @returns {Promise<any>}
*/
  createTopic(members: (string)[], name?: string, icon?: string): Promise<any>;
/**
* Join a topic
* #Arguments
* * `topicId` - topic id
* * `message` - message
* * `source` - source
* @param {string} topicId
* @param {string | undefined} [message]
* @param {string | undefined} [source]
* @returns {Promise<void>}
*/
  joinTopic(topicId: string, message?: string, source?: string): Promise<void>;
/**
* Add user into topic
* #Arguments
* * `topicId` - topic id
* * `userId` - user id
* #Return
* * `TopicMember` || `undefined`
* @param {string} topicId
* @param {string} userId
* @returns {Promise<any>}
*/
  addMember(topicId: string, userId: string): Promise<any>;
/**
* Get topic info
* #Arguments
* * `topicId` - topic id
* #Return
* * `Topic` || `undefined`
* @param {string} topicId
* @returns {Promise<any>}
*/
  getTopic(topicId: string): Promise<any>;
/**
* Get topic admins
* #Arguments
* * `topicId` - topic id
* #Return
* * `Vec<User>` || `undefined`
* @param {string} topicId
* @returns {Promise<any>}
*/
  getTopicAdmins(topicId: string): Promise<any>;
/**
* Get topic owner
* #Arguments
* * `topicId` - topic id
* #Return
* * `User` || `undefined`
* @param {string} topicId
* @returns {Promise<any>}
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
* @param {string} topicId
* @param {string} updatedAt
* @param {number} limit
* @returns {Promise<any>}
*/
  getTopicMembers(topicId: string, updatedAt: string, limit: number): Promise<any>;
/**
* Get topic knocks
* #Arguments
* * `topicId` - topic id
* #Return
* * `Vec<TopicKnock>`
* @param {string} topicId
* @returns {Promise<any>}
*/
  getTopicKnocks(topicId: string): Promise<any>;
/**
* Update topic info
* #Arguments
* * `topicId` - topic id
* * `option` - option
*     * `name` - String
*     * `icon` - String (url) or base64
* @param {string} topicId
* @param {any} option
* @returns {Promise<void>}
*/
  updateTopic(topicId: string, option: any): Promise<void>;
/**
* Update topic notice
* #Arguments
* * `topicId` - topic id
* * `text` - notice text
* @param {string} topicId
* @param {string} text
* @returns {Promise<void>}
*/
  updateTopicNotice(topicId: string, text: string): Promise<void>;
/**
* Silence topic
* #Arguments
* * `topicId` - topic id
* * `duration` - duration, format: 1d, 1h, 1m, cancel with empty string
* @param {string} topicId
* @param {string | undefined} [duration]
* @returns {Promise<void>}
*/
  silentTopic(topicId: string, duration?: string): Promise<void>;
/**
* Silent topic member
* #Arguments
* * `topicId` - topic id
* * `userId` - user id
* * `duration` - duration, format: 1d, 1h, 1m, cancel with empty string
* @param {string} topicId
* @param {string} userId
* @param {string | undefined} [duration]
* @returns {Promise<void>}
*/
  silentTopicMember(topicId: string, userId: string, duration?: string): Promise<void>;
/**
* Add topic admin
* #Arguments
* * `topicId` - topic id
* * `userId` - user id
* @param {string} topicId
* @param {string} userId
* @returns {Promise<void>}
*/
  addTopicAdmin(topicId: string, userId: string): Promise<void>;
/**
* Remove topic admin
* #Arguments
* * `topicId` - topic id
* * `userId` - user id
* @param {string} topicId
* @param {string} userId
* @returns {Promise<void>}
*/
  removeTopicAdmin(topicId: string, userId: string): Promise<void>;
/**
* Transfer topic
* #Arguments
* * `topicId` - topic id
* * `userId` - user id to transfer, the user must be a topic member
* @param {string} topicId
* @param {string} userId
* @returns {Promise<void>}
*/
  transferTopic(topicId: string, userId: string): Promise<void>;
/**
* Quit topic
* #Arguments
* * `topicId` - topic id
* @param {string} topicId
* @returns {Promise<void>}
*/
  quitTopic(topicId: string): Promise<void>;
/**
* Dismiss topic
* #Arguments
* * `topicId` - topic id
* @param {string} topicId
* @returns {Promise<void>}
*/
  dismissTopic(topicId: string): Promise<void>;
/**
* Accept topic join
* #Arguments
* * `topicId` - topic id
* * `userId` - user id
* * `memo` - accept memo
* @param {string} topicId
* @param {string} userId
* @param {string | undefined} [memo]
* @returns {Promise<void>}
*/
  acceptTopicJoin(topicId: string, userId: string, memo?: string): Promise<void>;
/**
* Decline topic join
* #Arguments
* * `topicId` - topic id
* * `userId` - user id
* * `message` - decline message
* @param {string} topicId
* @param {string} userId
* @param {string | undefined} [message]
* @returns {Promise<void>}
*/
  declineTopicJoin(topicId: string, userId: string, message?: string): Promise<void>;
/**
* Remove topic member
* #Arguments
* * `topicId` - topic id
* * `userId` - user id
* @param {string} topicId
* @param {string} userId
* @returns {Promise<void>}
*/
  removeTopicMember(topicId: string, userId: string): Promise<void>;
/**
* get the current connection status
* return: connecting, connected, broken, shutdown
*/
  readonly connectionStatus: string;
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
  onbroken: any;
/**
* Set the callback when connection connected
*/
  onconnected: any;
/**
* Set the callback when connection connecting
*/
  onconnecting: any;
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
  onconversationsremoved: any;
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
  onconversationsupdated: any;
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
  onkickoff: any;
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
  onsystemrequest: any;
/**
* Set the callback when connection token expired
*/
  ontokenexpired: any;
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
  ontopicmessage: any;
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
  ontopicread: any;
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
  ontopictyping: any;
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
  onunknownrequest: any;
}
/**
*/
export class IntoUnderlyingByteSource {
  free(): void;
/**
* @param {any} controller
*/
  start(controller: any): void;
/**
* @param {any} controller
* @returns {Promise<any>}
*/
  pull(controller: any): Promise<any>;
/**
*/
  cancel(): void;
/**
*/
  readonly autoAllocateChunkSize: number;
/**
*/
  readonly type: string;
}
/**
*/
export class IntoUnderlyingSink {
  free(): void;
/**
* @param {any} chunk
* @returns {Promise<any>}
*/
  write(chunk: any): Promise<any>;
/**
* @returns {Promise<any>}
*/
  close(): Promise<any>;
/**
* @param {any} reason
* @returns {Promise<any>}
*/
  abort(reason: any): Promise<any>;
}
/**
*/
export class IntoUnderlyingSource {
  free(): void;
/**
* @param {any} controller
* @returns {Promise<any>}
*/
  pull(controller: any): Promise<any>;
/**
*/
  cancel(): void;
}
/**
* Raw options for [`pipeTo()`](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream/pipeTo).
*/
export class PipeOptions {
  free(): void;
/**
*/
  readonly preventAbort: boolean;
/**
*/
  readonly preventCancel: boolean;
/**
*/
  readonly preventClose: boolean;
/**
*/
  readonly signal: AbortSignal | undefined;
}
/**
*/
export class QueuingStrategy {
  free(): void;
/**
*/
  readonly highWaterMark: number;
}
/**
* Raw options for [`getReader()`](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream/getReader).
*/
export class ReadableStreamGetReaderOptions {
  free(): void;
/**
*/
  readonly mode: any;
}
