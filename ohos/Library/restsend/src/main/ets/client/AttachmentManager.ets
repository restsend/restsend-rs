import { Api, ApiError } from '../api/Api'
import { MediaApi, UploadRequest, UploadContentRequest } from '../net/MediaApi'
import { Upload, parseUpload } from '../model/Upload'
import { parseJson } from '../common/Json'

/**
 * Upload/download flow shared by all platforms; the byte moving happens in
 * the MediaApi adapter (node FormData / @ohos multipart).
 */
export class AttachmentManager {
  private media: MediaApi
  private api: Api

  constructor(media: MediaApi, api: Api) {
    this.media = media
    this.api = api
  }

  async upload(filePath: string, fileName: string,
    onProgress?: (sent: number, total: number) => void): Promise<Upload> {
    const req: UploadRequest = {
      url: this.api.endpoint + '/api/attachment/upload',
      headers: this.api.authHeaders(),
      filePath: filePath,
      fileName: fileName,
      onProgress: onProgress
    }
    const resp = await this.media.upload(req)
    if (resp.status !== 200) {
      throw new ApiError(resp.status, `upload failed: ${resp.body}`)
    }
    const json = parseJson(resp.body)
    if (json === null) {
      throw new ApiError(resp.status, 'invalid upload response')
    }
    return parseUpload(json)
  }

  async download(url: string, savePath: string): Promise<void> {
    await this.media.download(this.resolveUrl(url), this.api.authHeaders(), savePath)
  }

  /** upload an in-memory string (chat-history export) */
  async uploadContent(fileName: string, content: string): Promise<Upload> {
    const req: UploadContentRequest = {
      url: this.api.endpoint + '/api/attachment/upload',
      headers: this.api.authHeaders(),
      fileName: fileName,
      content: content
    }
    const resp = await this.media.uploadContent(req)
    if (resp.status !== 200) {
      throw new ApiError(resp.status, `upload failed: ${resp.body}`)
    }
    const json = parseJson(resp.body)
    if (json === null) {
      throw new ApiError(resp.status, 'invalid upload response')
    }
    return parseUpload(json)
  }

  /** attachment urls: absolute passthrough, /root-relative, or bare upload path */
  resolveUrl(url: string): string {
    if (url.startsWith('http://') || url.startsWith('https://') || url.startsWith('ws://')) {
      return url
    }
    const base = this.api.endpoint
    if (url.startsWith('/')) {
      return base + url
    }
    // bare upload path (backend returns "uuid-name.ext")
    return base + '/api/attachment/' + url
  }
}
