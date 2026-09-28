import { HttpResponse } from './Transport'

/** Parameters for one multipart upload. */
export interface UploadRequest {
  url: string
  headers: Record<string, string>
  /** platform-local file path */
  filePath: string
  fileName: string
  onProgress?: (sent: number, total: number) => void
}

export interface MediaApi {
  /** POST the file as multipart/form-data field "file"; returns the raw response */
  upload(req: UploadRequest): Promise<HttpResponse>
  /** POST an in-memory string (e.g. chat-history export) as multipart "file" */
  uploadContent(req: UploadContentRequest): Promise<HttpResponse>
  /** GET a (possibly private) attachment and write it to savePath */
  download(url: string, headers: Record<string, string>, savePath: string): Promise<void>
}

export interface UploadContentRequest {
  url: string
  headers: Record<string, string>
  fileName: string
  content: string
}
