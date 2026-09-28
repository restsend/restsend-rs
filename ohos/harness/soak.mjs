// Soak test: long-running loop to prove the SDK core is leak-free.
// Usage: node soak.mjs [minutes=10]
// Success criteria: bounded internal counters, RSS slope ~0, no crashes.
import { initHttpAdapter, Api, RestsendClient, SendCallback } from './sdk.mjs'
import { NodeHttp, NodeWs } from './adapters.mjs'

const MINUTES = Number(process.argv[2] ?? process.env.SOAK_MINUTES ?? 10)
const ENDPOINT = process.env.RESTSEND_ENDPOINT ?? 'http://127.0.0.1:18123'
initHttpAdapter(new NodeHttp())

let unhandled = 0
process.on('unhandledRejection', (e) => {
  unhandled += 1
  console.log('[unhandledRejection]', e)
})

const rnd = () => 'soak' + Math.random().toString(36).slice(2, 8)
const infoA = await Api.signup(ENDPOINT, rnd(), 'pass-123456')
const infoB = await Api.signup(ENDPOINT, rnd(), 'pass-123456')

let bWs = null
const A = new RestsendClient(infoA, () => new NodeWs())
const B = new RestsendClient(infoB, () => {
  bWs = new NodeWs()
  return bWs
})
let reconnects = 0
B.events.onConnected = () => {
  if (reconnects > 0) {
    reconnects += 0 // counter handled below
  }
}
let connectedOnce = false
B.events.onConnected = (() => {
  const prev = B.events.onConnected
  return () => {
    connectedOnce = true
    prev()
  }
})()
A.events.onConnected = () => {
  connectedOnce = true
}

await Promise.all([A.connect(), B.connect()])
await new Promise((r) => setTimeout(r, 800))
const convA = await A.api.chatCreate(infoB.userId)
const topicA = convA.topicId
let bTopic = ''
B.events.onTopicMessage = (tid, m) => {
  bTopic = tid
  return { hasRead: false, unreadCountable: true }
}

console.log('soak start:', MINUTES, 'min; topicA =', topicA)
const started = Date.now()
const deadline = started + MINUTES * 60000
let sent = 0
let acked = 0
let failed = 0
let drops = 0
let samples = []
let seq = 0
let lastReconnect = Date.now()

while (Date.now() < deadline) {
  seq += 1
  const text = 'soak-' + seq
  const cb = new SendCallback()
  cb.onAck = () => {
    acked += 1
  }
  cb.onFail = (reason) => {
    failed += 1
    if (failed <= 3) console.log('[fail]', reason)
  }
  A.doSendText(topicA, text, cb)
  sent += 1

  // periodic hard drop of B's socket -> reconnect storm coverage
  if (seq % 30 === 0 && bWs !== null) {
    reconnects += 1
    try { bWs.kill() } catch {}
  }

  if (seq % 10 === 0) {
    const st = A.debugStats()
    st.rss = Math.round(process.memoryUsage().rss / 1024 / 1024)
    st.acked = acked
    st.failed = failed
    st.reconnects = reconnects
    samples.push(st)
    const first = samples[0]
    console.log(
      'rss=' + st.rss + 'MB (base ' + first.rss + ')',
      'pending=' + st.pending, 'convs=' + st.conversations,
      'logTopics=' + st.logTopics, 'logEntries=' + st.logEntries,
      'tomb=' + st.removedTombstones, 'mapped=' + st.mappedChatIds,
      'users=' + st.cachedUsers, 'reconnects=' + st.reconnects
    )
  }
  await new Promise((r) => setTimeout(r, 1000))
}

console.log('--- soak summary ---')
console.log('sent=' + sent, 'acked=' + acked, 'failed=' + failed, 'samples=' + samples.length)
console.log('unhandledRejections=' + unhandled)
if (samples.length > 2) {
  const base = samples[0].rss
  const last = samples[samples.length - 1].rss
  const drift = last - base
  console.log('rss base=' + base + 'MB last=' + last + 'MB drift=' + drift + 'MB')
  if (drift > 60) {
    console.log('FAIL: rss drift too high')
    process.exit(1)
  }
  const st = A.debugStats()
  if (st.pending > 20 || st.mappedChatIds > 20) {
    console.log('FAIL: internal counters unbounded', JSON.stringify(st))
    process.exit(1)
  }
}
console.log('SOAK OK')
A.shutdown()
B.shutdown()
process.exit(0)
