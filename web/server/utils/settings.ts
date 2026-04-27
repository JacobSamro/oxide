// Typed wrappers around the Setting key/value store.
// Defaults are returned for unset keys so the UI can edit a meaningful starting form.
import { get, run } from './db'

export interface DomainSettings {
  primaryDomain: string         // e.g. registry.example.com
  extraDomains: string[]        // additional SANs
  publicUrl: string             // base URL clients hit (used for tarball URL rewrite)
}

export interface SslSettings {
  enabled: boolean
  acmeEmail: string             // contact email for Let's Encrypt
  staging: boolean              // use Let's Encrypt staging directory
  httpRedirect: boolean         // redirect :80 → :443
}

export interface S3Settings {
  enabled: boolean
  endpoint: string              // optional: blank = AWS standard
  region: string
  bucket: string
  accessKey: string
  secretKey: string
  pathPrefix: string            // e.g. tarballs/
  pathStyle: boolean            // true for MinIO etc.
}

const DEFAULTS = {
  domain: <DomainSettings>{ primaryDomain: '', extraDomains: [], publicUrl: 'http://localhost:4873' },
  ssl:    <SslSettings>{ enabled: false, acmeEmail: '', staging: true, httpRedirect: true },
  s3:     <S3Settings>{ enabled: false, endpoint: '', region: 'us-east-1', bucket: '', accessKey: '', secretKey: '', pathPrefix: 'tarballs/', pathStyle: false },
}

type Key = keyof typeof DEFAULTS

export function readSetting<K extends Key>(key: K): typeof DEFAULTS[K] {
  const row = get<any>('SELECT value FROM Setting WHERE key = ?', [key])
  if (!row) return DEFAULTS[key]
  try { return { ...DEFAULTS[key], ...JSON.parse(row.value) } } catch { return DEFAULTS[key] }
}

export function writeSetting<K extends Key>(key: K, value: typeof DEFAULTS[K]) {
  const json = JSON.stringify(value)
  run(
    `INSERT INTO Setting (key, value, updatedAt) VALUES (?, ?, datetime('now'))
     ON CONFLICT(key) DO UPDATE SET value = excluded.value, updatedAt = excluded.updatedAt`,
    [key, json],
  )
}

export function readAllSettings() {
  return {
    domain: readSetting('domain'),
    ssl:    readSetting('ssl'),
    s3:     readSetting('s3'),
  }
}
