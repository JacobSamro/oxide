// Lightweight logger; mirrors levels used throughout the server.
const ts = () => new Date().toISOString()

const fmt = (level: string, msg: string, ctx?: Record<string, unknown>) => {
  const base = `[${ts()}] ${level} ${msg}`
  return ctx && Object.keys(ctx).length ? `${base} ${JSON.stringify(ctx)}` : base
}

export const logger = {
  info(msg: string, ctx?: Record<string, unknown>) { console.log(fmt('INFO', msg, ctx)) },
  warn(msg: string, ctx?: Record<string, unknown>) { console.warn(fmt('WARN', msg, ctx)) },
  error(msg: string, ctx?: Record<string, unknown>) { console.error(fmt('ERROR', msg, ctx)) },
  debug(msg: string, ctx?: Record<string, unknown>) {
    if (process.env.DEBUG) console.log(fmt('DEBUG', msg, ctx))
  },
}
