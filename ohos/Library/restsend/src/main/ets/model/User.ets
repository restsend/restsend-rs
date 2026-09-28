import { Json, jstr, jbool, jobj } from '../common/Json'

/** Free-form key/value map used by extra fields. */
export type Extra = Record<string, Object>

/** Login result. Same fields as the Rust `AuthInfo` record. */
export interface AuthInfo {
  endpoint: string
  userId: string
  avatar: string
  name: string
  token: string
  isStaff?: boolean
  isCrossDomain?: boolean
  privateExtra?: Extra
}

export function authInfo(endpoint: string, userId: string, token: string): AuthInfo {
  const info: AuthInfo = {
    endpoint: endpoint,
    userId: userId,
    avatar: '',
    name: '',
    token: token
  }
  return info
}

export function parseAuthInfo(o: Json): AuthInfo {
  const info: AuthInfo = {
    endpoint: jstr(o, 'endpoint'),
    userId: jstr(o, 'userId'),
    avatar: jstr(o, 'avatar'),
    name: jstr(o, 'name'),
    token: jstr(o, 'token'),
    isStaff: jbool(o, 'isStaff'),
    isCrossDomain: jbool(o, 'isCrossDomain')
  }
  const extra = jobj(o, 'privateExtra')
  if (extra !== undefined) {
    info.privateExtra = extra
  }
  return info
}

export interface User {
  userId: string
  name: string
  avatar: string
  publicKey?: string
  remark?: string
  isContact?: boolean
  isStar?: boolean
  isBlocked?: boolean
  locale?: string
  city?: string
  country?: string
  isStaff?: boolean
  extra?: Extra
}

export function parseUser(o: Json): User {
  const user: User = {
    userId: jstr(o, 'userId'),
    name: jstr(o, 'name'),
    avatar: jstr(o, 'avatar'),
    publicKey: jstr(o, 'publicKey'),
    remark: jstr(o, 'remark'),
    isContact: jbool(o, 'isContact'),
    isStar: jbool(o, 'isStar'),
    isBlocked: jbool(o, 'isBlocked'),
    isStaff: jbool(o, 'isStaff')
  }
  return user
}
