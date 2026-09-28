import { router } from '@kit.ArkUI'
import { Conversation, Content, ContentType } from 'restsend'
import { Session } from '../common/Session'

function previewOf(c: Content | undefined): ResourceStr {
  if (c === undefined) {
    return ''
  }
  if (c.type === ContentType.Image) {
    return $r('app.string.msg_image')
  }
  if (c.type === ContentType.Voice) {
    return $r('app.string.msg_voice')
  }
  if (c.type === ContentType.Video) {
    return $r('app.string.msg_video')
  }
  if (c.type === ContentType.File) {
    return $r('app.string.msg_file')
  }
  return c.text ?? ''
}

function titleOf(c: Conversation): string {
  if (c.name !== '') {
    return c.name
  }
  if (c.remark !== undefined && c.remark !== '') {
    return c.remark
  }
  // 1:1 alias "me:peer" -> show the peer id
  const idx = c.topicId.indexOf(':')
  if (idx > 0) {
    return c.topicId.substring(idx + 1)
  }
  return c.topicId
}

@Component
struct ChatItem {
  title: string = ''
  preview: ResourceStr = ''
  unread: number = 0
  onOpen: () => void = () => {}

  build() {
    Row({ space: 12 }) {
      Text(this.title.substring(0, 1).toUpperCase())
        .width(44)
        .height(44)
        .borderRadius(22)
        .backgroundColor('#4a90d9')
        .fontColor(Color.White)
        .textAlign(TextAlign.Center)
        .fontSize(20)
      Column({ space: 4 }) {
        Text(this.title)
          .fontSize(16)
          .maxLines(1)
        Text(this.preview)
          .fontSize(13)
          .fontColor('#888888')
          .maxLines(1)
      }
      .layoutWeight(1)
      .alignItems(HorizontalAlign.Start)

      if (this.unread > 0) {
        Text(String(this.unread))
          .fontSize(12)
          .padding({ left: 6, right: 6 })
          .height(18)
          .borderRadius(9)
          .backgroundColor('#e5484d')
          .fontColor(Color.White)
          .textAlign(TextAlign.Center)
      }
    }
    .width('100%')
    .padding({ left: 16, right: 16, top: 10, bottom: 10 })
    .onClick(() => this.onOpen())
  }
}

@Entry
@Component
struct ChatsPage {
  @State items: Conversation[] = []
  @State status: string = ''
  private updatedHandler: ((conversations: Conversation[], total: number) => void) | null = null

  aboutToAppear(): void {
    const client = Session.client
    if (client === null) {
      router.replaceUrl({ url: 'pages/Index' })
      return
    }
    this.items = client.conversations()
    this.status = client.connectionStatus()
    // page-local handlers (previous ones are restored on leave)
    const events = client.events
    const prevUpdated = events.onConversationsUpdated
    const prevConnected = events.onConnected
    const prevBroken = events.onBroken
    this.updatedHandler = prevUpdated
    events.onConversationsUpdated = (conversations: Conversation[], total: number): void => {
      this.items = conversations
      prevUpdated(conversations, total)
    }
    events.onConnected = (): void => {
      this.status = 'connected'
      prevConnected()
    }
    events.onBroken = (reason: string): void => {
      this.status = 'broken'
      prevBroken(reason)
    }
    client.syncConversations().catch((): void => {})
  }

  aboutToDisappear(): void {
    const client = Session.client
    if (client !== null && this.updatedHandler !== null) {
      client.events.onConversationsUpdated = this.updatedHandler
    }
  }

  open(item: Conversation): void {
    Session.openTopic = item.topicId
    router.pushUrl({ url: 'pages/Chat', params: { topicId: item.topicId } })
  }

  build() {
    Column() {
      Row() {
        Text($r('app.string.chats_title'))
          .fontSize(20)
          .fontWeight(FontWeight.Bold)
        Blank()
        Text(this.status === 'connected' ? $r('app.string.status_online') : $r('app.string.status_offline'))
          .fontSize(12)
          .fontColor(this.status === 'connected' ? '#30a46c' : '#999999')
      }
      .width('100%')
      .padding({ left: 16, right: 16, top: 14, bottom: 10 })

      List({ space: 2 }) {
        ForEach(this.items, (item: Conversation) => {
          ListItem() {
            ChatItem({
              title: titleOf(item),
              preview: previewOf(item.lastMessage),
              unread: item.unread,
              onOpen: (): void => this.open(item)
            })
          }
        }, (item: Conversation) => item.topicId + ':' + String(item.unread) + ':' + item.updatedAt)
      }
      .layoutWeight(1)
      .divider({ strokeWidth: 0.5, color: '#f0f0f0', startMargin: 16, endMargin: 16 })
    }
    .width('100%')
    .height('100%')
  }
}
