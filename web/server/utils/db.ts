// Single shared bun:sqlite database. Schema is applied on first open.
// Bun's sqlite is synchronous and well-suited to a server like this.
import { Database } from 'bun:sqlite'
import { readFileSync } from 'node:fs'
import { mkdirSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { logger } from './logger'

let db: Database | null = null

export function getDb(): Database {
  if (db) return db
  const cfg = useRuntimeConfig()
  const path = resolve(cfg.sqlitePath)
  mkdirSync(dirname(path), { recursive: true })
  db = new Database(path, { create: true, strict: true })
  db.exec('PRAGMA journal_mode = WAL')
  db.exec('PRAGMA foreign_keys = ON')

  // Apply schema (idempotent — every CREATE is IF NOT EXISTS).
  const schema = readFileSync(resolve(process.cwd(), 'server/db/schema.sql'), 'utf8')
  db.exec(schema)

  logger.info('sqlite ready', { path })
  return db
}

export function all<T = any>(sql: string, params: any = []): T[] {
  const stmt = getDb().query(sql)
  return stmt.all(...(Array.isArray(params) ? params : [params])) as T[]
}

export function get<T = any>(sql: string, params: any = []): T | undefined {
  const stmt = getDb().query(sql)
  return stmt.get(...(Array.isArray(params) ? params : [params])) as T | undefined
}

export function run(sql: string, params: any = []): { lastInsertRowid: number; changes: number } {
  const stmt = getDb().query(sql)
  const res: any = stmt.run(...(Array.isArray(params) ? params : [params]))
  return { lastInsertRowid: Number(res.lastInsertRowid ?? 0), changes: Number(res.changes ?? 0) }
}
