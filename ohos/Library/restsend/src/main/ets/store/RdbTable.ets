import { relationalStore } from '@kit.ArkData'
import { Context } from '@kit.AbilityKit'
import { Row, Table } from './Table'

const TABLE = 'rows'
const COLUMNS = ['partition', 'key', 'sortBy', 'value']

/**
 * Table implementation on @ohos.data.relationalStore (SQLite).
 * One store per account database; a single `rows` table keeps the
 * (partition, key, sortBy, value-json) model shared with the core.
 */
export class RdbTable implements Table {
  private storePromise: Promise<relationalStore.RdbStore>

  constructor(context: Context, dbName: string) {
    this.storePromise = this.open(context, dbName)
  }

  private async open(context: Context, dbName: string): Promise<relationalStore.RdbStore> {
    const config: relationalStore.StoreConfig = {
      name: dbName + '.db',
      securityLevel: relationalStore.SecurityLevel.S1
    }
    const store = await relationalStore.getRdbStore(context, config)
    await store.executeSql(
      'CREATE TABLE IF NOT EXISTS rows (' +
      'partition TEXT NOT NULL, key TEXT NOT NULL, sortBy INTEGER NOT NULL, value TEXT NOT NULL, ' +
      'PRIMARY KEY(partition, key))')
    return store
  }

  private async store(): Promise<relationalStore.RdbStore> {
    return this.storePromise
  }

  async get(partition: string, key: string): Promise<string | undefined> {
    const store = await this.store()
    const pred = new relationalStore.RdbPredicates(TABLE)
    pred.equalTo('partition', partition).and().equalTo('key', key)
    const rs = await store.query(pred, ['value'])
    let value: string | undefined = undefined
    if (rs.goToNextRow()) {
      value = rs.getString(rs.getColumnIndex('value'))
    }
    rs.close()
    return value
  }

  async set(partition: string, key: string, sortBy: number, value: string): Promise<void> {
    const store = await this.store()
    const bucket: relationalStore.ValuesBucket = {
      'partition': partition, 'key': key, 'sortBy': sortBy, 'value': value
    }
    await store.insert(TABLE, bucket, relationalStore.ConflictResolution.ON_CONFLICT_REPLACE)
  }

  async remove(partition: string, key: string): Promise<void> {
    const store = await this.store()
    const pred = new relationalStore.RdbPredicates(TABLE)
    pred.equalTo('partition', partition).and().equalTo('key', key)
    await store.delete(pred)
  }

  async query(partition: string, limit: number, desc: boolean): Promise<Row[]> {
    const store = await this.store()
    const pred = new relationalStore.RdbPredicates(TABLE)
    pred.equalTo('partition', partition)
    if (desc) {
      pred.orderByDesc('sortBy')
    } else {
      pred.orderByAsc('sortBy')
    }
    pred.limitAs(limit)
    const rs = await store.query(pred, COLUMNS)
    const rows: Row[] = []
    while (rs.goToNextRow()) {
      rows.push({
        partition: rs.getString(rs.getColumnIndex('partition')),
        key: rs.getString(rs.getColumnIndex('key')),
        sortBy: rs.getLong(rs.getColumnIndex('sortBy')),
        value: rs.getString(rs.getColumnIndex('value'))
      })
    }
    rs.close()
    return rows
  }

  async clearPartition(partition: string): Promise<void> {
    const store = await this.store()
    const pred = new relationalStore.RdbPredicates(TABLE)
    pred.equalTo('partition', partition)
    await store.delete(pred)
  }
}
