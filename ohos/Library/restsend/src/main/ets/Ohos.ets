import { Context } from '@kit.AbilityKit'
import fs from '@ohos.file.fs'
import { util } from '@kit.ArkTS'
import { AuthInfo } from './model/User'
import { ClientOptions } from './client/ClientOptions'
import { RestsendClient } from './client/RestsendClient'
import { initHttpAdapter } from './api/Api'
import { OhosHttp, OhosWs, OhosMedia } from './net/OhosAdapter'
import { RdbTable } from './store/RdbTable'

/**
 * One-call bootstrap for device apps:
 *
 *   const client = createRestsendClient(this.context, info, info.userId)
 *   client.events.onConnected = (): void => { ... }
 *   await client.connect()
 */
export function createRestsendClient(context: Context, info: AuthInfo, dbName: string,
  opts?: ClientOptions): RestsendClient {
  initHttpAdapter(new OhosHttp())
  const store = new RdbTable(context, dbName)
  return new RestsendClient(info, () => new OhosWs(), store, opts, new OhosMedia())
}

/**
 * Multi-account support, parity with rust account.rs: a `.current_user`
 * marker per app + one db per account (dbName = `restsend_<userId>`).
 */
export class OhosAccounts {
  private static markerPath(context: Context): string {
    return context.filesDir + '/.current_user'
  }

  /** remember the logged-in account (creates its directory marker) */
  static setCurrent(context: Context, userId: string): void {
    fs.writeSync(fs.openSync(OhosAccounts.markerPath(context),
      fs.OpenMode.READ_WRITE | fs.OpenMode.CREATE | fs.OpenMode.TRUNC).fd, userId)
  }

  /** '' when no account was recorded */
  static getCurrent(context: Context): string {
    const path = OhosAccounts.markerPath(context)
    if (!fs.accessSync(path)) {
      return ''
    }
    const stat: fs.Stat = fs.statSync(path)
    const buf = new ArrayBuffer(stat.size)
    fs.readSync(fs.openSync(path, fs.OpenMode.READ_ONLY).fd, buf)
    return String(new util.TextDecoder().decodeWithStream(new Uint8Array(buf)))
  }
}
