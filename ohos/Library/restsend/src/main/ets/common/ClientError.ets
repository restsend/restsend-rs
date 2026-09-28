/**
 * Error model aligned with the Rust `ClientError` enum (17 variants).
 * `kind` carries the rust variant name; HTTP status maps to the closest one.
 */
export enum ClientErrorKind {
  InvalidPassword = 'InvalidPassword',
  Forbidden = 'Forbidden',
  TokenExpired = 'TokenExpired',
  NetworkBroken = 'NetworkBroken',
  TopicNotFound = 'TopicNotFound',
  TopicKnockNotFound = 'TopicKnockNotFound',
  ChatLogNotFound = 'ChatLogNotFound',
  InvalidContent = 'InvalidContent',
  ConversationNotFound = 'ConversationNotFound',
  UserNotFound = 'UserNotFound',
  KickOffByOtherClient = 'KickOffByOtherClient',
  StdError = 'StdError',
  WebsocketError = 'WebsocketError',
  HTTP = 'HTTP',
  UserCancel = 'UserCancel',
  Storage = 'Storage',
  Other = 'Other'
}

export class ClientError extends Error {
  readonly kind: ClientErrorKind
  /** http status when the error came from a response, else 0 */
  readonly status: number

  constructor(kind: ClientErrorKind, message: string, status: number = 0) {
    super(message)
    this.kind = kind
    this.status = status
  }

  /** map an http status + body message to the rust error kinds */
  static fromStatus(status: number, message: string): ClientError {
    if (status === 401) {
      return new ClientError(ClientErrorKind.TokenExpired, message || 'auth: invalid token', status)
    }
    if (status === 403) {
      return new ClientError(ClientErrorKind.Forbidden, message || 'auth: forbidden', status)
    }
    if (status === 404) {
      return new ClientError(ClientErrorKind.HTTP, `not found: ${message}`, status)
    }
    if (status === 429) {
      return new ClientError(ClientErrorKind.HTTP, 'too many requests', status)
    }
    return new ClientError(ClientErrorKind.HTTP, message || `http ${status}`, status)
  }

  static network(message: string): ClientError {
    return new ClientError(ClientErrorKind.NetworkBroken, message)
  }

  static cancel(message: string = 'cancel: user cancel'): ClientError {
    return new ClientError(ClientErrorKind.UserCancel, message)
  }
}
