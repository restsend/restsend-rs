import { router } from '@kit.ArkUI'
import { ChatLog, ChatLogStatus } from 'restsend'
import { Session } from '../common/Session'

const PAGE_LOGS = 50

@Entry
@Component
struct ChatPage {
  @State logs: ChatLog[] = []
  @State input: string = ''
  @State typing: ResourceStr = ''
  @State title: string = ''
  private topicId: string = ''
  private scroller: Scroller = new Scroller()
  private typingTimer: number = -1
  private savedLogsChanged: ((topicId: string) => void) | null = null
  private savedTyping: ((topicId: string, message: string) => void) | null = null

  aboutToAppear(): void {
    const client = Session.client
    if (client === null) {
      router.back()
      return
    }
    const params = router.getParams() as Record<string, string>
    this.topicId = params['topicId'] ?? ''
    const conv = client.conversation(this.topicId)
    this.title = conv !== undefined && conv.name !== '' ? conv.name : this.peerId()
    this.refresh()
    const events = client.events
    this.savedLogsChanged = events.onLogsChanged
    this.savedTyping = events.onTopicTyping
    events.onLogsChanged = (topicId: string): void => {
      if (topicId === this.topicId) {
        this.refresh()
      }
      if (this.savedLogsChanged !== null) {
        this.savedLogsChanged(topicId)
      }
    }
    events.onTopicTyping = (topicId: string, message: string): void => {
      if (topicId === this.topicId) {
        this.showTyping()
      }
      if (this.savedTyping !== null) {
        this.savedTyping(topicId, message)
      }
    }
    client.setConversationRead(this.topicId).catch((): void => {})
    client.syncChatLogs(this.topicId, 0).catch((): void => {})
  }

  aboutToDisappear(): void {
    Session.openTopic = ''
    const client = Session.client
    if (client !== null) {
      if (this.savedLogsChanged !== null) {
        client.events.onLogsChanged = this.savedLogsChanged
      }
      if (this.savedTyping !== null) {
        client.events.onTopicTyping = this.savedTyping
      }
    }
    if (this.typingTimer >= 0) {
      clearTimeout(this.typingTimer)
    }
  }

  private peerId(): string {
    const idx = this.topicId.indexOf(':')
    return idx > 0 ? this.topicId.substring(idx + 1) : this.topicId
  }

  private refresh(): void {
    const client = Session.client
    if (client === null) {
      return
    }
    this.logs = client.getLogs(this.topicId, PAGE_LOGS)
    setTimeout((): void => this.scroller.scrollEdge(Edge.Bottom), 50)
  }

  private showTyping(): void {
    this.typing = $r('app.string.peer_typing')
    if (this.typingTimer >= 0) {
      clearTimeout(this.typingTimer)
    }
    this.typingTimer = setTimeout((): void => {
      this.typing = ''
    }, 3000)
  }

  send(): void {
    const client = Session.client
    const text = this.input.trim()
    if (client === null || text === '') {
      return
    }
    this.input = ''
    client.doSendText(this.topicId, text)
  }

  isMine(log: ChatLog): boolean {
    return log.senderId === (Session.info?.userId ?? '')
  }


  build() {
    Column() {
      Row({ space: 8 }) {
        Text('<')
          .fontSize(20)
          .onClick(() => router.back())
          .padding({ left: 12, right: 4 })
        Text(this.title)
          .fontSize(18)
          .fontWeight(FontWeight.Medium)
          .layoutWeight(1)
        if (this.typing !== '') {
          Text(this.typing)
            .fontSize(12)
            .fontColor('#999999')
            .margin({ right: 12 })
        }
      }
      .width('100%')
      .padding({ top: 12, bottom: 10 })

      List({ scroller: this.scroller, space: 8 }) {
        ForEach(this.logs, (log: ChatLog) => {
          ListItem() {
            Row() {
              if (this.isMine(log)) {
                Blank()
                this.Bubble(log)
              } else {
                this.Bubble(log)
                Blank()
              }
            }
            .width('100%')
            .padding({ left: 12, right: 12 })
          }
        }, (log: ChatLog) => log.id + ':' + String(log.seq) + ':' + log.status + ':' + String(log.recall))
      }
      .layoutWeight(1)

      Row({ space: 8 }) {
        TextInput({ placeholder: $r('app.string.input_hint'), text: this.input })
          .layoutWeight(1)
          .onChange((v: string) => this.input = v)
          .onSubmit(() => this.send())
        Button($r('app.string.btn_send'))
          .onClick(() => this.send())
      }
      .width('100%')
      .padding(10)
    }
    .width('100%')
    .height('100%')
  }

  @Builder
  Bubble(log: ChatLog) {
    if (log.recall) {
      Text($r('app.string.msg_recalled'))
        .fontSize(15)
        .fontColor('#999999')
        .backgroundColor('#f2f2f2')
        .padding({ left: 12, right: 12, top: 8, bottom: 8 })
        .borderRadius(12)
        .constraintSize({ maxWidth: '78%' })
    } else {
      Text((log.content.text ?? '') + (this.isMine(log) && log.status === ChatLogStatus.SendFailed ? ' !' : ''))
        .fontSize(15)
        .fontColor(Color.White)
        .backgroundColor(this.isMine(log) ? '#4a90d9' : '#e6e6e6')
        .padding({ left: 12, right: 12, top: 8, bottom: 8 })
        .borderRadius(12)
        .constraintSize({ maxWidth: '78%' })
    }
  }
}
