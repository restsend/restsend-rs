const CHAT_ID_LEN = 10
const CHAT_ID_CHARS = 'abcdefghijklmnopqrstuvwxyz0123456789'

/** Random chat id, same shape as the Rust core (10 lowercase alnum chars). */
export function randomChatId(): string {
  let s = ''
  for (let i = 0; i < CHAT_ID_LEN; i++) {
    s += CHAT_ID_CHARS.charAt(Math.floor(Math.random() * CHAT_ID_CHARS.length))
  }
  return s
}

/** 4-char nonce used in the websocket handshake url. */
export function randomNonce(): string {
  let s = ''
  for (let i = 0; i < 4; i++) {
    s += CHAT_ID_CHARS.charAt(Math.floor(Math.random() * CHAT_ID_CHARS.length))
  }
  return s
}

export function nowMillis(): number {
  return Date.now()
}
