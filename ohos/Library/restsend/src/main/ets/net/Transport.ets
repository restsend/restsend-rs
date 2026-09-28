/**
 * Transport interfaces. The SDK core only depends on these; platform
 * adapters (@ohos for devices, node for tests) implement them.
 */
export interface HttpResponse {
  status: number
  body: string
}

export interface HttpApi {
  request(method: string, url: string, headers: Record<string, string>, body: string): Promise<HttpResponse>
}

/**
 * Events delivered by a websocket connection. Subclass or assign the
 * function fields after construction.
 */
export class WsEvents {
  onOpen: () => void = (): void => {}
  onMessage: (text: string) => void = (text: string): void => {}
  /** called when an established connection drops */
  onClose: (reason: string) => void = (reason: string): void => {}
  /** called for connect failures and transport errors */
  onError: (reason: string) => void = (reason: string): void => {}
}

export interface WsTransport {
  /** resolves with the handshake http status (e.g. 101/200 ok, 401 unauthorized) */
  connect(url: string, headers: Record<string, string>, events: WsEvents): Promise<number>
  send(text: string): void
  close(): void
}

export const SDK_VERSION = '0.1.0'

export function connectUrl(endpoint: string, version: string, nonce: string): string {
  const base = endpoint.replace(/^http/, 'ws')
  return `${base}/api/connect?device=harmony&nonce=${nonce}&version=${version}`
}
