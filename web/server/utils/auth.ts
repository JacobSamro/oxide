// Session-based auth. Sessions live in DB; cookie holds the session id.
import { randomBytes } from 'node:crypto'
import bcrypt from 'bcryptjs'
import type { H3Event } from 'h3'
import { all, get, run } from './db'
import { logger } from './logger'

const COOKIE = 'oxide_sid'
const TTL_DAYS = 30

export interface SessionUser {
  id: number
  email: string
  name: string
  isAdmin: boolean
}

export async function hashPassword(plain: string) {
  return bcrypt.hash(plain, 10)
}

export async function verifyPassword(plain: string, hash: string) {
  return bcrypt.compare(plain, hash)
}

export async function createSession(event: H3Event, userId: number) {
  const id = randomBytes(32).toString('hex')
  const expiresAt = new Date(Date.now() + TTL_DAYS * 86400 * 1000).toISOString().replace('T', ' ').slice(0, 19)
  run('INSERT INTO Session (id, userId, expiresAt) VALUES (?, ?, ?)', [id, userId, expiresAt])
  setCookie(event, COOKIE, id, {
    httpOnly: true,
    sameSite: 'lax',
    path: '/',
    maxAge: TTL_DAYS * 86400,
  })
  logger.info('session created', { userId })
}

export async function destroySession(event: H3Event) {
  const sid = getCookie(event, COOKIE)
  if (sid) run('DELETE FROM Session WHERE id = ?', [sid])
  deleteCookie(event, COOKIE, { path: '/' })
}

export async function currentUser(event: H3Event): Promise<SessionUser | null> {
  const sid = getCookie(event, COOKIE)
  if (!sid) return null
  const row = get<any>(
    `SELECT u.id, u.email, u.name, u.isAdmin
       FROM Session s
       JOIN User u ON u.id = s.userId
      WHERE s.id = ? AND s.expiresAt > datetime('now')`,
    [sid],
  )
  if (!row) return null
  return { id: row.id, email: row.email, name: row.name, isAdmin: !!row.isAdmin }
}

export async function requireUser(event: H3Event): Promise<SessionUser> {
  const u = await currentUser(event)
  if (!u) throw createError({ statusCode: 401, statusMessage: 'Unauthorized', data: { success: false, message: 'Login required' } })
  return u
}

export async function requireAdmin(event: H3Event) {
  const u = await requireUser(event)
  if (!u.isAdmin) throw createError({ statusCode: 403, statusMessage: 'Forbidden', data: { success: false, message: 'Admin only' } })
  return u
}
