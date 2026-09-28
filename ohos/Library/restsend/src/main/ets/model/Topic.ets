import { Json, jstr, jnum, jstrArr, jarr, jbool } from '../common/Json'
import { User, parseUser } from './User'

export interface Topic {
  id: string
  name: string
  icon: string
  kind: string
  remark: string
  ownerId: string
  attendeeId: string
  admins: string[]
  members: number
  createdAt: string
  updatedAt: string
  notice?: string
  extra?: Record<string, Object>
}

export function parseTopic(o: Json): Topic {
  const t: Topic = {
    id: jstr(o, 'id'),
    name: jstr(o, 'name'),
    icon: jstr(o, 'icon'),
    kind: jstr(o, 'kind'),
    remark: jstr(o, 'remark'),
    ownerId: jstr(o, 'ownerId'),
    attendeeId: jstr(o, 'attendeeId'),
    admins: jstrArr(o, 'admins'),
    members: jnum(o, 'members'),
    createdAt: jstr(o, 'createdAt'),
    updatedAt: jstr(o, 'updatedAt'),
    notice: jstr(o, 'notice')
  }
  return t
}

export interface TopicMembersResult {
  hasMore: boolean
  updatedAt: string
  items: User[]
}

export function parseTopicMembers(o: Json): TopicMembersResult {
  const r: TopicMembersResult = {
    hasMore: jbool(o, 'hasMore'),
    updatedAt: jstr(o, 'updatedAt'),
    items: jarr(o, 'items').map(parseUser)
  }
  return r
}
