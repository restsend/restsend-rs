/**
 * Minimal storage abstraction: rows are (partition, key, sortBy, value-json).
 * ChatLog uses topicId as partition and seq as sortBy; Conversation uses ''.
 */
export interface Row {
  partition: string
  key: string
  sortBy: number
  value: string
}

export interface Table {
  get(partition: string, key: string): Promise<string | undefined>
  set(partition: string, key: string, sortBy: number, value: string): Promise<void>
  remove(partition: string, key: string): Promise<void>
  /** rows of one partition, ordered by sortBy */
  query(partition: string, limit: number, desc: boolean): Promise<Row[]>
  clearPartition(partition: string): Promise<void>
}

export class MemoryTable implements Table {
  private rows: Map<string, Row> = new Map()

  private k(partition: string, key: string): string {
    return partition + '\x00' + key
  }

  async get(partition: string, key: string): Promise<string | undefined> {
    const r = this.rows.get(this.k(partition, key))
    return r === undefined ? undefined : r.value
  }

  async set(partition: string, key: string, sortBy: number, value: string): Promise<void> {
    this.rows.set(this.k(partition, key), { partition: partition, key: key, sortBy: sortBy, value: value })
  }

  async remove(partition: string, key: string): Promise<void> {
    this.rows.delete(this.k(partition, key))
  }  async query(partition: string, limit: number, desc: boolean): Promise<Row[]> {
    const out: Row[] = []
    this.rows.forEach((r) => {
      if (r.partition === partition) {
        out.push(r)
      }
    })
    out.sort((a, b) => desc ? b.sortBy - a.sortBy : a.sortBy - b.sortBy)
    return out.slice(0, limit)
  }

  async clearPartition(partition: string): Promise<void> {
    const doomed: string[] = []
    this.rows.forEach((r, k) => {
      if (r.partition === partition) {
        doomed.push(k)
      }
    })
    doomed.forEach((k) => this.rows.delete(k))
  }
}
