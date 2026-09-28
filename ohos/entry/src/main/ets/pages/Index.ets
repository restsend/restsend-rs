import { router } from '@kit.ArkUI'
import { common } from '@kit.AbilityKit'
import { Session } from '../common/Session'

@Entry
@Component
struct LoginPage {
  @State endpoint: string = ''
  @State email: string = ''
  @State password: string = ''
  @State error: string = ''
  @State busy: boolean = false

  aboutToAppear(): void {
    const context = getContext(this) as common.Context
    Session.loadEndpoint(context).then((v: string) => {
      if (v !== '') {
        this.endpoint = v
      }
    })
  }

  async go(action: 'login' | 'signup' | 'guest'): Promise<void> {
    if (this.busy) {
      return
    }
    this.busy = true
    this.error = ''
    try {
      const context = getContext(this) as common.Context
      if (action === 'login') {
        await Session.login(context, this.endpoint.trim(), this.email.trim(), this.password)
      } else if (action === 'signup') {
        await Session.signup(context, this.endpoint.trim(), this.email.trim(), this.password)
      } else {
        await Session.guest(context, this.endpoint.trim())
      }
      Session.client?.connect()
      router.replaceUrl({ url: 'pages/Chats' })
    } catch (e) {
      this.error = String(e)
    } finally {
      this.busy = false
    }
  }

  build() {
    Column({ space: 14 }) {
      Text('RestSend')
        .fontSize(30)
        .fontWeight(FontWeight.Bold)
        .margin({ top: 60, bottom: 24 })

      TextInput({ placeholder: $r('app.string.endpoint_hint'), text: this.endpoint })
        .width('86%')
        .onChange((v: string) => this.endpoint = v)

      TextInput({ placeholder: $r('app.string.account_hint'), text: this.email })
        .width('86%')
        .onChange((v: string) => this.email = v)

      TextInput({ placeholder: $r('app.string.password_hint'), text: this.password })
        .type(InputType.Password)
        .width('86%')
        .onChange((v: string) => this.password = v)

      if (this.error !== '') {
        Text(this.error)
          .fontSize(13)
          .fontColor('#e5484d')
          .width('86%')
      }

      Button($r('app.string.btn_login'))
        .width('86%')
        .enabled(!this.busy)
        .onClick(() => this.go('login'))

      Row({ space: 12 }) {
        Button($r('app.string.btn_register'))
          .layoutWeight(1)
          .backgroundColor('#f0f0f0')
          .fontColor('#333333')
          .enabled(!this.busy)
          .onClick(() => this.go('signup'))
        Button($r('app.string.btn_guest'))
          .layoutWeight(1)
          .backgroundColor('#f0f0f0')
          .fontColor('#333333')
          .enabled(!this.busy)
          .onClick(() => this.go('guest'))
      }
      .width('86%')
    }
    .width('100%')
    .height('100%')
    .alignItems(HorizontalAlign.Center)
  }
}
