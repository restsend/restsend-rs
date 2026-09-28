import { Json, jstr, jnum, jbool } from '../common/Json'

/** Upload result returned by POST /api/attachment/upload. */
export interface Upload {
  path: string
  fileName: string
  thumbnail: string
  ext: string
  size: number
  external?: boolean
}

export function parseUpload(o: Json): Upload {
  const u: Upload = {
    path: jstr(o, 'path'),
    fileName: jstr(o, 'fileName'),
    thumbnail: jstr(o, 'thumbnail'),
    ext: jstr(o, 'ext'),
    size: jnum(o, 'size'),
    external: jbool(o, 'external')
  }
  return u
}
