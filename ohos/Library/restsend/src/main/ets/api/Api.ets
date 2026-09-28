import { HttpApi, HttpResponse } from '../net/Transport'
import { Json, parseJson, jstr, jnum, jbool, jarr, jobj } from '../common/Json'
import { ClientError, ClientErrorKind } from '../common/ClientError'
import { AuthInfo, User, parseUser } from '../model/User'
import { Conversation, parseConversation } from '../model/Conversation'
import { ChatRequest, parseChatRequest } from '../model/ChatRequest'
import { Topic, parseTopic, TopicMembersResult, parseTopicMembers } from '../model/Topic'
import { Upload, parseUpload } from '../model/Upload'

/** group creation options supported by the backend create form */
export interface TopicCreateOptions {
  kind?: string
  multiple?: boolean
  knockNeedVerify?: boolean
  privateTopic?: boolean
  ensureConversation?: boolean
}

/** backwards-compatible alias */
export class ApiError extends ClientError {
  constructor(status: number, message: string) {
    super(ClientErrorKind.HTTP, message, status)
  }
}

let httpAdapter: HttpApi | null = null

/** Platform entry must call this once (ohos adapter in device apps, node adapter in tests). */
export function initHttpAdapter(http: HttpApi): void {
  httpAdapter = http
}

function adapter(): HttpApi {
  if (httpAdapter === null) {
    throw new ClientError(ClientErrorKind.HTTP, 'http adapter not initialized')
  }
  return httpAdapter
}

export interface ChatListResult {
  total: number
  hasMore: boolean
  items: Conversation[]
  removed: RemovedConversation[]
  lastUpdatedAt: string
  lastRemovedAt: string
}

export interface RemovedConversation {
  topicId: string
  seq: number
}

export interface ChatSyncResult {
  topicId: string
  hasMore: boolean
  lastSeq: number
  items: ChatRequest[]
}

/** Thin REST wrapper. auth endpoints have no /api prefix, all others do. */
export class Api {
  readonly endpoint: string
  token: string

  constructor(endpoint: string, token: string = '') {
    this.endpoint = endpoint.replace(/\/+$/, '')
    this.token = token
  }

  private url(path: string): string {
    // auth paths are mounted at root; guest login already includes /api
    const prefix = path.startsWith('/auth') || path.startsWith('/api') ? '' : '/api'
    return this.endpoint + prefix + path
  }

  private headers(): Record<string, string> {
    const h: Record<string, string> = { 'Content-Type': 'application/json' }
    if (this.token !== '') {
      h['Authorization'] = `Bearer ${this.token}`
    }
    return h
  }

  /** bearer headers for media upload/download (no Content-Type) */
  authHeaders(): Record<string, string> {
    const h: Record<string, string> = {}
    if (this.token !== '') {
      h['Authorization'] = `Bearer ${this.token}`
    }
    return h
  }

  private async post(path: string, body: Object | null): Promise<Json> {
    const resp = await adapter().request(
      'POST', this.url(path), this.headers(), body === null ? '' : JSON.stringify(body))
    return this.decode(resp)
  }

  private async get(path: string): Promise<Json> {
    const resp = await adapter().request('GET', this.url(path), this.headers(), '')
    return this.decode(resp)
  }

  private decode(resp: HttpResponse): Json {
    const json = parseJson(resp.body)
    if (resp.status !== 200) {
      const msg = json !== null ? jstr(json, 'message', jstr(json, 'error', `http ${resp.status}`)) : `http ${resp.status}`
      throw ClientError.fromStatus(resp.status, msg)
    }
    if (json === null) {
      throw ClientError.fromStatus(resp.status, 'invalid json response')
    }
    return json
  }

  // ---- auth (usable before a client exists) ----

  private static async signin(endpoint: string, path: string, body: Object): Promise<AuthInfo> {
    const api = new Api(endpoint)
    const resp = await adapter().request('POST', api.url(path), api.headers(), JSON.stringify(body))
    const json = api.decode(resp)
    const profile = jobj(json, 'profile')
    const info: AuthInfo = {
      endpoint: api.endpoint,
      userId: jstr(json, 'email'),
      avatar: profile !== undefined ? jstr(profile, 'avatar') : '',
      name: jstr(json, 'displayName'),
      token: jstr(json, 'token'),
      isStaff: jbool(json, 'isStaff')
    }
    return info
  }

  static login(endpoint: string, email: string, password: string): Promise<AuthInfo> {
    const body: Record<string, Object> = { 'email': email, 'password': password, 'remember': true }
    return Api.signin(endpoint, '/auth/login', body)
  }

  static loginWithToken(endpoint: string, email: string, token: string): Promise<AuthInfo> {
    const body: Record<string, Object> = { 'token': token, 'remember': true }
    return Api.signin(endpoint, '/auth/login', body)
  }

  static signup(endpoint: string, email: string, password: string): Promise<AuthInfo> {
    const body: Record<string, Object> = { 'email': email, 'password': password, 'remember': true }
    return Api.signin(endpoint, '/auth/register', body)
  }

  static guestLogin(endpoint: string, guestId: string): Promise<AuthInfo> {
    const body: Record<string, Object> = { 'guestId': guestId, 'remember': true }
    return Api.signin(endpoint, '/api/guest/login', body)
  }

  async logout(): Promise<void> {
    await this.get('/auth/logout')
  }

  // ---- profile / relation ----

  async getProfile(userId: string): Promise<User> {
    return parseUser(await this.post(`/profile/${userId}`, null))
  }

  async getProfiles(userIds: string[]): Promise<User[]> {
    const body: Record<string, Object> = { 'ids': userIds }
    const json = await this.post('/profile', body)
    return jarr(json, 'items').map(parseUser)
  }

  async setUserRemark(userId: string, remark: string): Promise<void> {
    const body: Record<string, Object> = { 'remark': remark }
    await this.post(`/relation/${userId}`, body)
  }

  async setUserStar(userId: string, star: boolean): Promise<void> {
    const body: Record<string, Object> = { 'favorite': star }
    await this.post(`/relation/${userId}`, body)
  }

  async setUserBlock(userId: string, blocked: boolean): Promise<void> {
    await this.post(blocked ? `/block/${userId}` : `/unblock/${userId}`, null)
  }

  // ---- conversations ----

  async chatList(limit: number, updatedAt: string, lastRemovedAt: string): Promise<ChatListResult> {
    const body: Record<string, Object> = { 'offset': 0, 'limit': limit }
    if (updatedAt !== '') {
      body['updatedAt'] = updatedAt
    }
    if (lastRemovedAt !== '') {
      body['lastRemovedAt'] = lastRemovedAt
    }
    const json = await this.post('/chat/list', body)
    const r: ChatListResult = {
      total: jnum(json, 'total'),
      hasMore: jbool(json, 'hasMore'),
      items: jarr(json, 'items').map(parseConversation),
      removed: jarr(json, 'removed').map((o: Json): RemovedConversation => {
        return { topicId: jstr(o, 'topicId', jstr(o, 'id')), seq: jnum(o, 'seq') }
      }),
      lastUpdatedAt: jstr(json, 'lastUpdatedAt'),
      lastRemovedAt: jstr(json, 'lastRemovedAt')
    }
    return r
  }

  async chatCreate(userId: string): Promise<Conversation> {
    return parseConversation(await this.post(`/chat/create/${userId}`, null))
  }

  async chatInfo(topicId: string): Promise<Conversation> {
    return parseConversation(await this.post(`/chat/info/${topicId}`, null))
  }

  async chatSync(topicId: string, lastSeq: number, limit: number): Promise<ChatSyncResult> {
    const body: Record<string, Object> = { 'limit': limit }
    if (lastSeq > 0) {
      body['lastSeq'] = lastSeq
    }
    const json = await this.post(`/chat/sync/${topicId}`, body)
    const r: ChatSyncResult = {
      topicId: jstr(json, 'topicId', topicId),
      hasMore: jbool(json, 'hasMore'),
      lastSeq: jnum(json, 'lastSeq'),
      items: jarr(json, 'items').map(parseChatRequest)
    }
    return r
  }

  /** one request refreshing many conversations: [{topicId,lastSeq,limit},...] */
  async chatBatchSync(topics: Record<string, Object>[]): Promise<ChatSyncResult[]> {
    const out: ChatSyncResult[] = []
    const json = await this.post('/chat/batch_sync', topics as Object)
    const pages: Json[] = []
    if (Array.isArray(json)) {
      for (const item of json as Array<Object>) {
        pages.push(item as Json)
      }
    } else {
      const nested = jarr(json, 'items')
      for (const item of nested) {
        pages.push(item)
      }
    }
    for (const item of pages) {
      out.push({
        topicId: jstr(item, 'topicId'),
        hasMore: jbool(item, 'hasMore'),
        lastSeq: jnum(item, 'lastSeq'),
        items: jarr(item, 'items').map(parseChatRequest)
      })
    }
    return out
  }

  async chatRead(topicId: string, seq: number): Promise<void> {
    const body: Record<string, Object> = { 'seq': seq }
    await this.post(`/chat/read/${topicId}`, body)
  }

  async chatReadAll(): Promise<void> {
    await this.post('/chat/readall', null)
  }

  /** mark a conversation as (un)read without touching seq */
  async chatUnread(topicId: string, unread: boolean): Promise<void> {
    const body: Record<string, Object> = { 'unread': unread }
    await this.post(`/chat/unread/${topicId}`, body)
  }

  async chatClearMessages(topicId: string): Promise<void> {
    await this.post(`/chat/clear_messages/${topicId}`, null)
  }

  async chatRemoveMessages(topicId: string, chatIds: string[]): Promise<void> {
    const body: Record<string, Object> = { 'ids': chatIds }
    await this.post(`/chat/remove_messages/${topicId}`, body)
  }

  async setAllowGuestChat(allow: boolean): Promise<void> {
    const body: Record<string, Object> = { 'allowGuest': allow }
    await this.post('/profile/update', body)
  }

  async chatUpdate(topicId: string, fields: Record<string, Object>): Promise<void> {
    await this.post(`/chat/update/${topicId}`, fields)
  }

  async chatRemove(topicId: string): Promise<void> {
    await this.post(`/chat/remove/${topicId}`, null)
  }

  // ---- topics (groups) ----

  async topicCreate(name: string, memberIds: string[], icon: string = '',
    opts?: TopicCreateOptions): Promise<Topic> {
    const body: Record<string, Object> = { 'name': name, 'members': memberIds }
    if (icon !== '') {
      body['icon'] = icon
    }
    if (opts !== undefined) {
      if (opts.kind !== undefined) {
        body['kind'] = opts.kind
      }
      if (opts.multiple !== undefined) {
        body['multiple'] = opts.multiple
      }
      if (opts.knockNeedVerify !== undefined) {
        body['knockNeedVerify'] = opts.knockNeedVerify
      }
      if (opts.privateTopic !== undefined) {
        body['private'] = opts.privateTopic
      }
      if (opts.ensureConversation !== undefined) {
        body['ensureConversation'] = opts.ensureConversation
      }
    }
    return parseTopic(await this.post('/topic/create', body))
  }

  async topicInfo(topicId: string): Promise<Topic> {
    return parseTopic(await this.post(`/topic/info/${topicId}`, null))
  }

  async topicMembers(topicId: string, limit: number = 100): Promise<TopicMembersResult> {
    const body: Record<string, Object> = { 'limit': limit }
    return parseTopicMembers(await this.post(`/topic/members/${topicId}`, body))
  }

  async topicInvite(topicId: string, userId: string): Promise<void> {
    await this.post(`/topic/invite/${topicId}/${userId}`, null)
  }

  async topicKnock(topicId: string, message: string, source: string = ''): Promise<void> {
    const body: Record<string, Object> = { 'message': message }
    if (source !== '') {
      body['source'] = source
    }
    await this.post(`/topic/knock/${topicId}`, body)
  }

  async topicQuit(topicId: string): Promise<void> {
    await this.post(`/topic/quit/${topicId}`, null)
  }

  async topicDismiss(topicId: string): Promise<void> {
    await this.post(`/topic/dismiss/${topicId}`, null)
  }

  async topicNotice(topicId: string, notice: string): Promise<void> {
    const body: Record<string, Object> = { 'notice': notice }
    await this.post(`/topic/admin/notice/${topicId}`, body)
  }

  async topicKick(topicId: string, userId: string): Promise<void> {
    await this.post(`/topic/admin/kickout/${topicId}/${userId}`, null)
  }

  async topicAddAdmin(topicId: string, userId: string): Promise<void> {
    await this.post(`/topic/admin/add_admin/${topicId}/${userId}`, null)
  }

  async topicRemoveAdmin(topicId: string, userId: string): Promise<void> {
    await this.post(`/topic/admin/remove_admin/${topicId}/${userId}`, null)
  }

  async topicTransfer(topicId: string, userId: string): Promise<void> {
    await this.post(`/topic/admin/transfer/${topicId}/${userId}`, null)
  }

  async topicSilentMember(topicId: string, userId: string, duration: string = ''): Promise<void> {
    const body: Record<string, Object> = { 'duration': duration }
    await this.post(`/topic/admin/silent_topic/${topicId}/${userId}`, body)
  }

  async topicSilent(topicId: string, duration: string): Promise<void> {
    const body: Record<string, Object> = { 'duration': duration }
    await this.post(`/topic/admin/silent_topic/${topicId}`, body)
  }

  async topicAddMember(topicId: string, userId: string): Promise<void> {
    await this.post(`/topic/admin/add_member/${topicId}/${userId}`, null)
  }

  async topicUpdate(topicId: string, name: string, icon: string): Promise<void> {
    const body: Record<string, Object> = {}
    if (name !== '') {
      body['name'] = name
    }
    if (icon !== '') {
      body['icon'] = icon
    }
    await this.post(`/topic/admin/update/${topicId}`, body)
  }

  async topicUpdateNotice(topicId: string, text: string): Promise<void> {
    const body: Record<string, Object> = { 'notice': text }
    await this.post(`/topic/admin/notice/${topicId}`, body)
  }

  /** pending join requests (admin) */
  async topicKnockList(topicId: string): Promise<Array<Json>> {
    const json = await this.post(`/topic/admin/list_knock/${topicId}`, null)
    if (Array.isArray(json)) {
      return json as Json[]
    }
    return jarr(json, 'items')
  }

  async topicAcceptJoin(topicId: string, userId: string, memo: string = ''): Promise<void> {
    const body: Record<string, Object> = { 'memo': memo }
    await this.post(`/topic/admin/knock/accept/${topicId}/${userId}`, body)
  }

  async topicDeclineJoin(topicId: string, userId: string, message: string = ''): Promise<void> {
    const body: Record<string, Object> = { 'message': message }
    await this.post(`/topic/admin/knock/reject/${topicId}/${userId}`, body)
  }

  /** http fallback for sending one chat request */
  async chatSend(topicId: string, req: ChatRequest): Promise<Json> {
    return this.post(`/chat/send/${topicId}`, req as Object)
  }

  parseUpload(json: Json): Upload {
    return parseUpload(json)
  }
}
