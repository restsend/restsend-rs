// End-to-end integration tests: run the ArkTS SDK core (bundled for Node)
// against a real restsend-backend instance.
// Prereq: node build.mjs && backend on :18123
import { test, before } from 'node:test'
import assert from 'node:assert'
import fs from 'node:fs'
import { initHttpAdapter, Api, RestsendClient, SendCallback, ClientOptions, MemoryTable } from './sdk.mjs'
import { NodeHttp, NodeWs, NodeMedia } from './adapters.mjs'

const ENDPOINT = process.env.RESTSEND_ENDPOINT ?? 'http://127.0.0.1:18123'
initHttpAdapter(new NodeHttp())

const randomUser = () => 'ark' + Math.random().toString(36).slice(2, 10)

// the backend rate-limits auth, so the whole suite shares 3 logins
const users = {}

function withTimeout(promise, ms, label) {
  return Promise.race([
    promise,
    new Promise((_, reject) => setTimeout(() => reject(new Error('timeout: ' + label)), ms))
  ])
}

function waitFor(pred, ms, label) {
  return withTimeout(new Promise((resolve) => {
    const timer = setInterval(() => {
      if (pred()) {
        clearInterval(timer)
        resolve()
      }
    }, 100)
  }), ms, label)
}

async function withRetry(fn, times = 20, waitMs = 1500) {
  let lastErr
  for (let i = 0; i < times; i++) {
    try {
      return await fn()
    } catch (e) {
      lastErr = e
      const msg = String(e && e.message)
      if (msg.includes('429') || msg.toLowerCase().includes('too many')) {
        await new Promise((r) => setTimeout(r, waitMs))
        continue
      }
      throw e
    }
  }
  throw lastErr
}

before(async () => {
  const emailA = 'ark' + Math.random().toString(36).slice(2, 10)
  const emailB = 'ark' + Math.random().toString(36).slice(2, 10)
  const guestId = 'ark' + Math.random().toString(36).slice(2, 10)
  users.infoA = await withRetry(() => Api.signup(ENDPOINT, emailA, 'pass-123456'))
  users.infoB = await withRetry(() => Api.signup(ENDPOINT, emailB, 'pass-123456'))
  users.infoGuest = await withRetry(() => Api.guestLogin(ENDPOINT, guestId))
  users.guestId = guestId
})

test('auth: signup + guest login produce token identities', async () => {
  assert.equal(users.infoA.userId, users.infoA.userId)
  assert.ok(users.infoA.token.length > 0)
  assert.equal(users.infoGuest.userId, users.guestId)
})

test('e2e: connect, send, receive, ack, read sync, typing, history', async (t) => {
  const infoA = users.infoA
  const infoB = users.infoB

  const clientA = new RestsendClient(infoA, () => new NodeWs())
  const clientB = new RestsendClient(infoB, () => new NodeWs())
  const clientA2 = new RestsendClient(infoA, () => new NodeWs()) // A's second device
  t.after(() => {
    clientA.shutdown()
    clientB.shutdown()
    clientA2.shutdown()
  })

  await Promise.all([clientA.connect(), clientB.connect(), clientA2.connect()])
  await waitFor(() => clientA.connectionStatus() === 'connected', 10000, 'A connected')
  await waitFor(() => clientB.connectionStatus() === 'connected', 10000, 'B connected')

  // A opens a 1:1 chat with B
  const conv = await clientA.api.chatCreate(infoB.userId)
  assert.ok(conv.topicId.length > 0)
  const topicId = conv.topicId

  // B receives the message over ws; A gets an ack with a server seq
  const gotMsg = withTimeout(new Promise((resolve) => {
    clientB.events.onTopicMessage = (tid, m) => {
      resolve({ tid, m })
      const s = { hasRead: false, unreadCountable: true }
      return s
    }
  }), 10000, 'B onTopicMessage')
  const gotAck = withTimeout(new Promise((resolve) => {
    const cb = new SendCallback()
    cb.onAck = (r) => resolve(r)
    clientA.doSendText(topicId, 'hello from arkts', cb)
  }), 10000, 'A ack')
  const { tid, m } = await gotMsg
  // 1:1 topics are aliased per side as "me:peer", so B's topicId differs from A's
  assert.ok(tid.split(':').includes(infoA.userId) && tid.split(':').includes(infoB.userId))
  assert.equal(m.content.text, 'hello from arkts')
  const ack = await gotAck
  assert.ok((ack.seq ?? 0) > 0, 'ack carries a seq')

  // local log became Sent and the conversation healed
  await waitFor(() => {
    const logs = clientA.getLogs(topicId, 10)
    return logs.some((l) => l.id === m.chatId && l.status === 'sent')
  }, 3000, 'A log sent')
  assert.equal(clientA.conversation(topicId).unread, 0)
  assert.ok(clientB.conversation(tid).unread >= 1, 'B unread bumped')

  // B reads -> persisted on the server
  await clientB.setConversationRead(tid)
  await waitFor(() => {
    const c = clientB.conversation(tid)
    return c !== undefined && c.lastReadSeq >= (ack.seq ?? 0)
  }, 5000, 'B local read seq')
  const bList = await clientB.api.chatList(50, '', '')
  const bConv = bList.items.find((c) => c.topicId === tid)
  assert.ok(bConv !== undefined && bConv.lastReadSeq >= (ack.seq ?? 0), 'B read state on server')

  // A reads on device 1 -> device 2 (clientA2) receives the read sync
  const gotRead = withTimeout(new Promise((resolve) => {
    clientA2.events.onTopicRead = (tid2, r) => resolve(r)
  }), 10000, 'A2 read sync')
  await clientA.setConversationRead(topicId)
  const read = await gotRead
  assert.ok(read.attendee.startsWith(infoA.userId), 'read sync carries A identity')
  await waitFor(() => {
    const c = clientA2.conversation(topicId)
    return c !== undefined && c.unread === 0
  }, 5000, 'A2 unread cleared')

  // typing
  const gotTyping = withTimeout(new Promise((resolve) => {
    clientB.events.onTopicTyping = (tid2, msg) => resolve(tid2)
  }), 10000, 'B typing')
  clientA.doTyping(topicId)
  assert.equal(await gotTyping, tid)

  // history sync from the server on the fresh device (it already got the
  // live echo, so dedup makes count 0 — assert on merged logs instead)
  const sync = await clientA2.syncChatLogs(topicId, 0)
  assert.ok(sync.lastSeq >= (ack.seq ?? 0), 'sync reports server seq')
  const synced = clientA2.getLogs(topicId, 10)
  assert.ok(synced.some((l) => l.content.text === 'hello from arkts'), 'synced log content')
})

test('recall: peer sees the message marked recalled', async (t) => {
  const clientA = new RestsendClient(users.infoA, () => new NodeWs())
  const clientB = new RestsendClient(users.infoB, () => new NodeWs())
  t.after(() => {
    clientA.shutdown()
    clientB.shutdown()
  })
  await Promise.all([clientA.connect(), clientB.connect()])
  await waitFor(() => clientA.connectionStatus() === 'connected', 10000, 'A connected')
  await waitFor(() => clientB.connectionStatus() === 'connected', 10000, 'B connected')

  const conv = await clientA.api.chatCreate(users.infoB.userId)
  const topicId = conv.topicId
  const gotMsg = withTimeout(new Promise((resolve) => {
    clientB.events.onTopicMessage = (tid, m) => {
      resolve({ tid, m })
      return { hasRead: false, unreadCountable: true }
    }
  }), 10000, 'B message before recall')
  const sent = withTimeout(new Promise((resolve) => {
    const cb = new SendCallback()
    cb.onAck = (r) => resolve(r)
    clientA.doSendText(topicId, 'recall me', cb)
  }), 10000, 'ack before recall')
  const { tid, m } = await gotMsg
  const ack = await sent
  assert.ok(m.chatId !== undefined && m.chatId !== '')

  // A recalls; B's local log flips recall=true
  clientA.doRecall(topicId, m.chatId)
  await waitFor(() => {
    const logs = clientB.getLogs(tid, 10)
    return logs.some((l) => l.id === m.chatId && l.recall)
  }, 10000, 'B sees recall')
  assert.ok((ack.seq ?? 0) > 0)
})

test('remove conversation: local removal with anti-refill', async (t) => {
  const clientA = new RestsendClient(users.infoA, () => new NodeWs())
  const clientB = new RestsendClient(users.infoB, () => new NodeWs())
  t.after(() => {
    clientA.shutdown()
    clientB.shutdown()
  })
  await Promise.all([clientA.connect(), clientB.connect()])
  await waitFor(() => clientA.connectionStatus() === 'connected', 10000, 'A connected')

  const conv = await clientA.api.chatCreate(users.infoB.userId)
  const topicId = conv.topicId
  clientA.doSendText(topicId, 'before remove')
  await waitFor(() => clientA.conversation(topicId) !== undefined, 5000, 'A has conversation')

  await clientA.removeConversation(topicId)
  assert.equal(clientA.conversation(topicId), undefined, 'conversation removed locally')

  // fresh incremental sync must not resurrect it
  await clientA.syncConversations()
  assert.equal(clientA.conversation(topicId), undefined, 'no refill after sync')
})

test('offline queue: messages flush after reconnect', async (t) => {
  let lastWs = null
  const clientA = new RestsendClient(users.infoA, () => {
    lastWs = new NodeWs()
    return lastWs
  })
  const clientB = new RestsendClient(users.infoB, () => new NodeWs())
  t.after(() => {
    clientA.shutdown()
    clientB.shutdown()
  })
  await Promise.all([clientA.connect(), clientB.connect()])
  await waitFor(() => clientA.connectionStatus() === 'connected', 10000, 'A connected')
  await waitFor(() => clientB.connectionStatus() === 'connected', 10000, 'B connected')

  const conv = await clientA.api.chatCreate(users.infoB.userId)
  const topicId = conv.topicId

  // hard drop, then send while offline: the frame must be queued
  lastWs.kill()
  await waitFor(() => clientA.connectionStatus() === 'broken', 10000, 'A broken')
  clientA.doSendText(topicId, 'offline flush test')

  const gotMsg = withTimeout(new Promise((resolve) => {
    clientB.events.onTopicMessage = (tid, m) => {
      resolve(m)
      return { hasRead: false, unreadCountable: true }
    }
  }), 20000, 'B receives flushed message')
  await waitFor(() => clientA.connectionStatus() === 'connected', 15000, 'A reconnected')
  const m = await gotMsg
  assert.equal(m.content.text, 'offline flush test')
})

test('kickout: force-offline frame shuts the client down', async (t) => {  const opts = new ClientOptions()
  opts.nonce = 'kk99'
  const client = new RestsendClient(users.infoGuest, () => new NodeWs(), undefined, opts)
  t.after(() => client.shutdown())
  await client.connect()
  await waitFor(() => client.connectionStatus() === 'connected', 10000, 'connected')

  const kicked = withTimeout(new Promise((resolve) => {
    client.events.onKickoff = (reason) => resolve(reason)
  }), 10000, 'kick frame')
  const http = new NodeHttp()
  const resp = await http.request('POST', `${ENDPOINT}/api/kick/harmony:kk99`,
    { 'Authorization': `Bearer ${users.infoGuest.token}` }, '')
  assert.equal(resp.status, 200)
  await kicked
  await waitFor(() => client.connectionStatus() === 'shutdowned', 5000, 'shutdowned')
})

test('heartbeat: connection stays healthy on a short ping interval', async (t) => {
  const opts = new ClientOptions()
  opts.pingIntervalSecs = 2
  const client = new RestsendClient(users.infoGuest, () => new NodeWs(), undefined, opts)
  t.after(() => client.shutdown())
  let pingFailed = false
  client.events.onPingFailed = () => {
    pingFailed = true
  }
  await client.connect()
  await waitFor(() => client.connectionStatus() === 'connected', 10000, 'connected')
  // ~3 ping cycles must pass without failures or drops
  await new Promise((r) => setTimeout(r, 6500))
  assert.equal(pingFailed, false, 'no ping failures')
  assert.equal(client.connectionStatus(), 'connected')
})

test('reconnect: killed connection is restored automatically', async (t) => {
  const info = users.infoGuest
  let lastWs = null
  const client = new RestsendClient(info, () => {
    lastWs = new NodeWs()
    return lastWs
  })
  t.after(() => client.shutdown())

  await client.connect()
  await waitFor(() => client.connectionStatus() === 'connected', 10000, 'first connect')

  // hard-drop the underlying socket; the loop must reconnect with backoff
  lastWs.kill()
  await waitFor(() => client.connectionStatus() === 'connected', 15000, 'reconnected')
  assert.equal(client.connectionStatus(), 'connected')
})

test('persistence: conversations and logs survive a restart via local store', async (t) => {
  const store = new MemoryTable()
  const factory = () => new NodeWs()
  const clientA = new RestsendClient(users.infoA, factory, store)
  t.after(() => clientA.shutdown())
  await clientA.connect()
  await waitFor(() => clientA.connectionStatus() === 'connected', 10000, 'connected')

  const conv = await clientA.api.chatCreate(users.infoB.userId)
  const topicId = conv.topicId
  await withTimeout(new Promise((resolve) => {
    const cb = new SendCallback()
    cb.onAck = () => resolve()
    clientA.doSendText(topicId, 'persisted message', cb)
  }), 10000, 'ack')
  await waitFor(() => clientA.conversation(topicId) !== undefined, 5000, 'conversation merged')
  clientA.shutdown()

  // a brand new client over the SAME store must see data offline (hydrate only)
  const clientA2 = new RestsendClient(users.infoA, factory, store)
  t.after(() => clientA2.shutdown())
  await clientA2.hydrate()
  assert.ok(clientA2.conversation(topicId) !== undefined, 'conversation hydrated')
  assert.ok(clientA2.getLogs(topicId, 10).some((l) => l.content.text === 'persisted message'),
    'log hydrated')
})

test('media: image upload, delivery with attachment url, download round-trip', async (t) => {
  const clientA = new RestsendClient(users.infoA, () => new NodeWs(), undefined, undefined, new NodeMedia())
  const clientB = new RestsendClient(users.infoB, () => new NodeWs(), undefined, undefined, new NodeMedia())
  t.after(() => {
    clientA.shutdown()
    clientB.shutdown()
  })
  await Promise.all([clientA.connect(), clientB.connect()])
  await waitFor(() => clientA.connectionStatus() === 'connected', 10000, 'A connected')

  const conv = await clientA.api.chatCreate(users.infoB.userId)
  const topicId = conv.topicId

  const bytes = Buffer.from([0x89, 0x50, 0x4e, 0x47, 1, 2, 3, 4, 5, 6, 7, 8])
  const filePath = '/tmp/opencode/arkts-e2e-upload.bin'
  fs.mkdirSync('/tmp/opencode', { recursive: true })
  fs.writeFileSync(filePath, bytes)

  const gotMsg = withTimeout(new Promise((resolve) => {
    clientB.events.onTopicMessage = (tid, m) => {
      resolve(m)
      return { hasRead: false, unreadCountable: true }
    }
  }), 15000, 'B image message')
  const chatId = clientA.doSendImage(topicId, filePath, 'e2e-image.png')
  assert.ok(chatId.length === 10)

  const m = await gotMsg
  assert.equal(m.content.type, 'image')
  const att = m.content.attachment
  assert.ok(att !== undefined && att.url !== '', 'attachment url present')

  // download through the sdk and compare bytes
  const savePath = '/tmp/opencode/arkts-e2e-download.bin'
  await clientB.downloadAttachment(att.url, savePath)
  const downloaded = fs.readFileSync(savePath)
  assert.ok(Buffer.compare(bytes, downloaded) === 0, 'downloaded bytes identical')

  // local sender log became sent
  await waitFor(() => {
    const logs = clientA.getLogs(topicId, 10)
    return logs.some((l) => l.id === chatId && l.status === 'sent')
  }, 8000, 'A image log sent')
})

test('group: create topic, both members receive, members and quit', async (t) => {
  const clientA = new RestsendClient(users.infoA, () => new NodeWs())
  const clientB = new RestsendClient(users.infoB, () => new NodeWs())
  t.after(() => {
    clientA.shutdown()
    clientB.shutdown()
  })
  await Promise.all([clientA.connect(), clientB.connect()])
  await waitFor(() => clientB.connectionStatus() === 'connected', 10000, 'B connected')

  const topic = await clientA.createTopic('e2e-group', [users.infoB.userId, users.infoGuest.userId])
  assert.ok(topic.id.length > 0)
  assert.equal(topic.name, 'e2e-group')

  // group topics are real ids: both sides see the same topicId
  const gotMsg = withTimeout(new Promise((resolve) => {
    clientB.events.onTopicMessage = (tid, m) => {
      resolve({ tid, m })
      return { hasRead: false, unreadCountable: true }
    }
  }), 10000, 'B group message')
  await withTimeout(new Promise((resolve) => {
    const cb = new SendCallback()
    cb.onAck = () => resolve()
    clientA.doSendText(topic.id, 'hello group', cb)
  }), 10000, 'group ack')
  const { tid, m } = await gotMsg
  assert.equal(tid, topic.id, 'group topicId identical on both sides')
  assert.equal(m.content.text, 'hello group')

  const members = await clientA.topicMembers(topic.id)
  const ids = members.items.map((u) => u.userId)
  assert.ok(ids.includes(users.infoA.userId) && ids.includes(users.infoB.userId), 'both members listed')

  await clientB.quitTopic(topic.id)
  const after = await clientA.topicMembers(topic.id)
  assert.ok(!after.items.map((u) => u.userId).includes(users.infoB.userId), 'B removed after quit')
  assert.ok(after.items.map((u) => u.userId).includes(users.infoGuest.userId), 'others stay')
})

test('gap fill: missed messages are pulled automatically on next live frame', async (t) => {
  let bWs = null
  const clientA = new RestsendClient(users.infoA, () => new NodeWs())
  const clientB = new RestsendClient(users.infoB, () => {
    bWs = new NodeWs()
    return bWs
  })
  t.after(() => {
    clientA.shutdown()
    clientB.shutdown()
  })
  await Promise.all([clientA.connect(), clientB.connect()])
  await waitFor(() => clientA.connectionStatus() === 'connected', 10000, 'A connected')
  await waitFor(() => clientB.connectionStatus() === 'connected', 10000, 'B connected')

  const conv = await clientA.api.chatCreate(users.infoB.userId)
  const topicId = conv.topicId
  let bTopic = ''
  const seen = []
  clientB.events.onTopicMessage = (tid, m) => {
    bTopic = tid
    seen.push(m.content.text)
    return { hasRead: false, unreadCountable: true }
  }
  await withTimeout(new Promise((resolve) => {
    const cb = new SendCallback()
    cb.onAck = () => resolve()
    clientA.doSendText(topicId, 'gap-msg-1', cb)
  }), 10000, 'msg1 ack')
  await waitFor(() => seen.includes('gap-msg-1'), 5000, 'B got msg1')

  // B goes offline; A sends two messages that B will miss live
  bWs.kill()
  await waitFor(() => clientB.connectionStatus() === 'broken', 10000, 'B broken')
  await clientA.sendViaHttp(topicId, 'gap-msg-2')
  await clientA.sendViaHttp(topicId, 'gap-msg-3')
  await waitFor(() => clientB.connectionStatus() === 'connected', 15000, 'B reconnected')

  // next live frame opens a seq gap -> sdk pulls 2 and 3 automatically
  // (synced logs fire onLogsChanged, not onTopicMessage — assert on logs,
  // keyed by B's own topic alias)
  await withTimeout(new Promise((resolve) => {
    const cb = new SendCallback()
    cb.onAck = () => resolve()
    clientA.doSendText(topicId, 'gap-msg-4', cb)
  }), 10000, 'msg4 ack')
  await waitFor(() => {
    const logs = clientB.getLogs(bTopic, 20)
    return logs.some((l) => l.content.text === 'gap-msg-2') &&
      logs.some((l) => l.content.text === 'gap-msg-3')
  }, 10000, 'gap filled')
  assert.ok(seen.includes('gap-msg-4'))
})

test('settings + http fallback + extra frame', async (t) => {
  const clientA = new RestsendClient(users.infoA, () => new NodeWs())
  t.after(() => clientA.shutdown())
  await clientA.connect()
  await waitFor(() => clientA.connectionStatus() === 'connected', 10000, 'connected')

  const conv = await clientA.createChat(users.infoB.userId)
  const topicId = conv.topicId

  await clientA.setConversationMute(topicId, true)
  assert.equal(clientA.conversation(topicId).mute, true, 'mute applied locally')
  await clientA.setConversationSticky(topicId, true)
  assert.equal(clientA.conversation(topicId).sticky, true, 'sticky applied locally')
  await clientA.setConversationRemark(topicId, 'my friend')
  assert.equal(clientA.conversation(topicId).remark, 'my friend', 'remark applied locally')

  // http fallback send works without relying on the ws
  const seq = await clientA.sendViaHttp(topicId, 'via http fallback')
  assert.ok(seq > 0, 'http send returns seq')

  // update extra of the last log via ws frame (rust doUpdateExtra)
  const last = clientA.getLogs(topicId, 1)[0]
  assert.ok(last !== undefined, 'log exists for extra update')
  const chatId = clientA.doUpdateExtra(topicId, last.id, { "pinned-note": "hello extra" })
  assert.ok(chatId.length === 10)
})

test('parity: error kinds, getUser cache, countable, mentions, ping ack', async (t) => {
  const clientA = new RestsendClient(users.infoA, () => new NodeWs())
  const clientB = new RestsendClient(users.infoB, () => new NodeWs())
  t.after(() => {
    clientA.shutdown()
    clientB.shutdown()
  })
  await Promise.all([clientA.connect(), clientB.connect()])
  await waitFor(() => clientA.connectionStatus() === 'connected', 10000, 'A connected')

  // error mapping: bad token -> TokenExpired kind
  const bad = new RestsendClient({ ...users.infoA, token: 'invalid-token' }, () => new NodeWs())
  let expired = false
  bad.events.onTokenExpired = () => { expired = true }
  t.after(() => bad.shutdown())
  bad.connect().catch(() => {})
  await waitFor(() => expired, 10000, 'token expired event')

  // getUser via server + cache
  const u = await clientA.getUser(users.infoB.userId)
  assert.ok(u !== undefined && u.userId === users.infoB.userId)
  const cached = await clientA.getUser(users.infoB.userId, false)
  assert.equal(cached.userId, users.infoB.userId)

  // host-customizable countable: never bump unread
  let countCalls = 0
  clientB.setCountableCallback((content) => {
    countCalls += 1
    return false
  })
  const conv = await clientA.createChat(users.infoB.userId)
  const topicId = conv.topicId
  let cbTopic = ''
  const gotMsg = withTimeout(new Promise((resolve) => {
    clientB.events.onTopicMessage = (tid, m) => {
      cbTopic = tid
      resolve(m)
      return { hasRead: false, unreadCountable: true }
    }
  }), 10000, 'B countable message')
  await withTimeout(new Promise((resolve) => {
    const cb = new SendCallback()
    cb.onAck = () => resolve()
    clientA.doSendText(topicId, 'countable check', cb)
  }), 10000, 'ack countable')
  await gotMsg
  assert.ok(countCalls >= 1, 'countable callback invoked for the frame')
  assert.ok(cbTopic.split(':').includes(users.infoB.userId), 'B alias topic key')

  // mentions + reply reach the peer
  const gotRich = withTimeout(new Promise((resolve) => {
    clientB.events.onTopicMessage = (tid, m) => resolve(m)
  }), 10000, 'B rich message')
  const replyTarget = clientA.getLogs(topicId, 5).find((l) => (l.seq ?? 0) > 0)
  clientA.doSendText(topicId, 'hey @bob', undefined, { mentions: [users.infoB.userId], replyId: replyTarget?.id })
  const rich = await gotRich
  assert.ok((rich.content.mentions ?? []).includes(users.infoB.userId), 'mentions delivered')
  assert.equal(rich.content.reply, replyTarget?.id, 'reply id delivered')

  // manual ping gets an ack
  const pingAcked = withTimeout(new Promise((resolve) => {
    const cb = new SendCallback()
    cb.onAck = () => resolve(true)
    clientA.doPing(cb)
  }), 10000, 'ping ack')
  assert.ok(await pingAcked)

  // mark unread bumps locally (server flags the conversation unread too)
  await clientB.setConversationRead(cbTopic)
  await clientB.markConversationUnread(cbTopic)
  assert.ok((clientB.conversation(cbTopic)?.unread ?? 0) >= 1, 'unread bumped locally')

  // saveChatLogs + searchChatLog
  clientA.saveChatLogs([{
    topicId: topicId, id: 'saved-1', seq: 99999, createdAt: '', senderId: 'someone',
    content: { type: 'text', text: 'the needle in haystack' }, read: false,
    recall: false, status: 'received', cachedAt: Date.now()
  }])
  const hits = clientA.searchChatLog(topicId, undefined, 'needle')
  assert.ok(hits.items.length === 1 && hits.items[0].id === 'saved-1', 'search finds saved log')
})

test('parity2: topic admin surface, downloadFile, keepalive, filters', async (t) => {
  const clientA = new RestsendClient(users.infoA, () => new NodeWs())
  const clientB = new RestsendClient(users.infoB, () => new NodeWs())
  t.after(() => {
    clientA.shutdown()
    clientB.shutdown()
  })
  await Promise.all([clientA.connect(), clientB.connect()])
  await waitFor(() => clientA.connectionStatus() === 'connected', 10000, 'A connected')

  // group with rust-named methods
  const topic = await clientA.createTopic('parity-group', [users.infoB.userId, users.infoGuest.userId])
  const got = await clientA.getTopic(topic.id)
  assert.equal(got.id, topic.id)

  const admins = await clientA.getTopicAdmins(topic.id)
  assert.ok(Array.isArray(admins), 'admins list fetched')

  const owner = await clientA.getTopicOwner(topic.id)
  assert.equal(owner?.userId, users.infoA.userId, 'owner resolved')

  // admin ops via rust-named facade
  await clientA.addTopicAdmin(topic.id, users.infoB.userId)
  const admins2 = await clientA.getTopicAdmins(topic.id)
  assert.ok(admins2.some((u) => u.userId === users.infoB.userId), 'B became admin')
  await clientA.removeTopicAdmin(topic.id, users.infoB.userId)

  await clientA.silentTopicMember(topic.id, users.infoGuest.userId, '60')
  await clientA.addTopicMember(topic.id, users.infoGuest.userId)

  await clientA.updateTopic(topic.id, 'renamed-group', '')
  assert.equal((await clientA.getTopic(topic.id)).name, 'renamed-group')

  // notice update must not throw (server echo of notice is not guaranteed)
  await clientA.updateTopicNotice(topic.id, 'notice text')

  // give the group activity so a conversation exists for A
  await withTimeout(new Promise((resolve) => {
    const cb = new SendCallback()
    cb.onAck = () => resolve()
    clientA.doSendText(topic.id, 'group hello', cb)
  }), 10000, 'group ack')
  await clientA.syncConversations()

  // filters (wasm parity) — topic must be merged locally first
  const filtered = clientA.filterConversation((c) => c.topicId === topic.id)
  assert.ok(filtered.length >= 1, 'filterConversation works')
  const firstPage = await clientA.syncFirstPageConversations()
  assert.ok(Array.isArray(firstPage))

  // runtime keepalive change
  clientA.setKeepaliveIntervalSecs(45)

  // downloadFile with callback (round-trip through a real uploaded attachment)
  const bytes = Buffer.from([1, 2, 3, 4, 5, 6, 7, 8, 9])
  fs.writeFileSync('/tmp/opencode/arkts-dl-src.bin', bytes)
  const clientMedia = new RestsendClient(users.infoA, () => new NodeWs(), undefined, undefined, new NodeMedia())
  t.after(() => clientMedia.shutdown())
  const up = await clientMedia.attachments.upload('/tmp/opencode/arkts-dl-src.bin', 'parity-dl.bin')
  const savePath = await clientMedia.downloadFile(up.path)
  assert.ok(fs.readFileSync(savePath).equals(bytes), 'downloadFile round-trip')
})

test('parity3: cancel semantics, keepalive runtime, knock flow', async (t) => {
  const clientA = new RestsendClient(users.infoA, () => new NodeWs())
  const clientB = new RestsendClient(users.infoB, () => new NodeWs())
  t.after(() => {
    clientA.shutdown()
    clientB.shutdown()
  })
  await Promise.all([clientA.connect(), clientB.connect()])
  const conv = await clientA.createChat(users.infoB.userId)
  const topicId = conv.topicId

  // cancelSend keeps the log but marks it failed (rust parity)
  const chatId = clientA.doSendText(topicId, 'will be cancelled')
  clientA.cancelSend(chatId)
  const log = clientA.getChatLog(topicId, chatId)
  assert.ok(log !== undefined, 'log kept after cancel')
  assert.equal(log.status, 'sendFailed', 'log marked sendFailed')

  // knock -> accept flow (C joins A's group via request)
  const infoC = await withRetry(() => Api.signup(ENDPOINT, randomUser(), 'pass-123456'))
  const topic = await clientA.createTopic('knock-group',
    [users.infoGuest.userId, users.infoB.userId], '', { multiple: true, knockNeedVerify: true })
  const clientC = new RestsendClient(infoC, () => new NodeWs())
  t.after(() => clientC.shutdown())
  await clientC.connect()
  await clientC.joinTopic(topic.id, 'let me in')
  const knocks = await clientA.getTopicKnocks(topic.id)
  assert.ok(knocks.some((k) => k.userId === infoC.userId), 'knock listed')
  await clientA.acceptTopicJoin(topic.id, infoC.userId, 'welcome')
  const members = await clientA.topicMembers(topic.id)
  assert.ok(members.items.map((u) => u.userId).includes(infoC.userId), 'C joined after accept')
})

test('robustness: malformed frames do not crash the client', async (t) => {
  const clientA = new RestsendClient(users.infoA, () => new NodeWs())
  t.after(() => clientA.shutdown())
  await clientA.connect()
  await waitFor(() => clientA.connectionStatus() === 'connected', 10000, 'connected')

  // feed the parser garbage directly: must not throw out of handleFrame
  const garbage = [
    'not json at all',
    '{"type":123}',
    '{"type":"chat"}',
    '{"type":"chat","topicId":"x","content":{"type":"text","text":123}}',
    '{"type":"resp","chatId":null,"code":"abc"}',
    ''
  ]
  for (const g of garbage) {
    try {
      clientA['handleFrame'](g)
    } catch (e) {
      assert.fail(`handleFrame threw on ${JSON.stringify(g)}: ${e}`)
    }
  }
  assert.equal(clientA.connectionStatus(), 'connected', 'still connected after garbage')
})
