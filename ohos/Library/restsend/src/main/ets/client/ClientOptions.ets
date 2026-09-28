/** Runtime options with defaults copied from the Rust ClientOption. */
export class ClientOptions {
  /** seconds between keepalive pings */
  pingIntervalSecs: number = 30
  /** server clock diff beyond this is reported as ping failed */
  pingTimeoutSecs: number = 5
  /** resend count before a pending message fails */
  maxRetry: number = 2
  /** pending message older than this (secs) is resent or failed */
  maxSendIdleSecs: number = 20
  /** max reconnect backoff in seconds */
  maxConnectIntervalSecs: number = 5
  /** page size for /api/chat/sync */
  maxSyncLogsLimit: number = 100
  /** how many recent chat logs are kept in memory per topic */
  maxLogsLimit: number = 100
  /** user profile cache lifetime in seconds */
  userCacheExpireSecs: number = 60
  /** sdk version reported in the ws handshake */
  version: string = '0.1.0'
  /** fixed handshake nonce (mainly for tests / deterministic device ids) */
  nonce: string = ''
  /** conversations kept locally (oldest pruned beyond this) */
  maxConversationLimit: number = 1000
  /** chat logs older than this many days are cleaned up */
  messageRetentionDays: number = 30
  /** hard cap on queued+unacked sends (oldest is failed beyond this) */
  maxPendingSends: number = 1000
  /** queued (never sent) messages older than this are failed */
  queuedLifetimeSecs: number = 300
  /** removed-conversation tombstones expire after this (rust parity) */
  removedCacheTtlSecs: number = 300
  /** max user profiles kept in the memory cache */
  maxCachedUsers: number = 500
  /** local dir for downloaded files (device: context.cacheDir) */
  tempDir: string = ''
}
