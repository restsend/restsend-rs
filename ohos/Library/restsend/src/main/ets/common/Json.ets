/**
 * Defensive JSON access helpers.
 * All wire JSON is parsed into Record<string, Object> first, then
 * converted to typed models field by field (unknown fields are ignored).
 */
export type Json = Record<string, Object>

export function parseJson(text: string): Json | null {
  try {
    const v: Object | null = JSON.parse(text)
    if (v === null || v === undefined) {
      return null
    }
    return v as Json
  } catch (e) {
    return null
  }
}

export function jstr(o: Json, key: string, def: string = ''): string {
  const v = o[key]
  if (typeof v === 'string') {
    return v as string
  }
  return def
}

export function jnum(o: Json, key: string, def: number = 0): number {
  const v = o[key]
  if (typeof v === 'number') {
    return v as number
  }
  return def
}

export function jbool(o: Json, key: string, def: boolean = false): boolean {
  const v = o[key]
  if (typeof v === 'boolean') {
    return v as boolean
  }
  return def
}

export function jobj(o: Json, key: string): Json | undefined {
  const v = o[key]
  if (v !== null && v !== undefined && typeof v === 'object' && !Array.isArray(v)) {
    return v as Json
  }
  return undefined
}

export function jarr(o: Json, key: string): Array<Json> {
  const v = o[key]
  const out: Array<Json> = []
  if (Array.isArray(v)) {
    for (const item of v as Array<Object>) {
      if (item !== null && item !== undefined && typeof item === 'object' && !Array.isArray(item)) {
        out.push(item as Json)
      }
    }
  }
  return out
}

export function jstrArr(o: Json, key: string): Array<string> {
  const v = o[key]
  const out: Array<string> = []
  if (Array.isArray(v)) {
    for (const item of v as Array<Object>) {
      if (typeof item === 'string') {
        out.push(item as string)
      }
    }
  }
  return out
}
