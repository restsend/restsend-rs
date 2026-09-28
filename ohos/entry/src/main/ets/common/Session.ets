import { Api, AuthInfo, ChatRequest, ClientOptions, MessageStatus,
  RestsendClient, createRestsendClient, initHttpAdapter, OhosHttp } from 'restsend'
import { common } from '@kit.AbilityKit'
import { preferences } from '@kit.ArkData'

const PREFS = 'restsend_session'
const KEY_ENDPOINT = 'endpoint'

/** App-wide session: one client, bound to the logged-in account. */
export class Session {
  static client: RestsendClient | null = null
  static info: AuthInfo | null = null
  /** the topic currently open in the chat page; messages for it auto-read */
  static openTopic: string = ''

  static async saveEndpoint(context: common.Context, endpoint: string): Promise<void> {
    const prefs = await preferences.getPreferences(context, PREFS)
    await prefs.put(KEY_ENDPOINT, endpoint)
    await prefs.flush()
  }

  static async loadEndpoint(context: common.Context): Promise<string> {
    const prefs = await preferences.getPreferences(context, PREFS)
    const v = await prefs.get(KEY_ENDPOINT, '') as string
    return v
  }

  static async login(context: common.Context, endpoint: string, email: string, password: string): Promise<void> {
    initHttpAdapter(new OhosHttp())
    const info: AuthInfo = await Api.login(endpoint, email, password)
    await Session.saveEndpoint(context, endpoint)
    Session.bind(context, info)
  }

  static async signup(context: common.Context, endpoint: string, email: string, password: string): Promise<void> {
    initHttpAdapter(new OhosHttp())
    const info: AuthInfo = await Api.signup(endpoint, email, password)
    await Session.saveEndpoint(context, endpoint)
    Session.bind(context, info)
  }

  static async guest(context: common.Context, endpoint: string): Promise<void> {
    initHttpAdapter(new OhosHttp())
    const guestId = 'ohos' + Math.random().toString(36).slice(2, 10)
    const info: AuthInfo = await Api.guestLogin(endpoint, guestId)
    await Session.saveEndpoint(context, endpoint)
    Session.bind(context, info)
  }

  private static bind(context: common.Context, info: AuthInfo): void {
    Session.info = info
    const opts = new ClientOptions()
    const client = createRestsendClient(context, info, 'restsend_' + info.userId, opts)
    client.events.onTopicMessage = (topicId: string, message: ChatRequest): MessageStatus => {
      const s: MessageStatus = { hasRead: topicId === Session.openTopic, unreadCountable: true }
      return s
    }
    Session.client = client
  }

  static logout(): void {
    if (Session.client !== null) {
      Session.client.shutdown()
      Session.client = null
    }
    Session.info = null
    Session.openTopic = ''
  }

  /** called from UIAbility.onForeground */
  static wake(): void {
    Session.client?.appActive()
  }
}
