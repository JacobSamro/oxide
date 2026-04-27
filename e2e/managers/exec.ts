// Tiny helper around Bun.spawn for running PM commands and capturing exit code + duration.
import { spawn } from 'bun'

export interface RunResult { exitCode: number; durationMs: number; stdout: string; stderr: string }

export async function run(cmd: string[], opts: { cwd?: string; env?: Record<string, string>; timeoutMs?: number } = {}): Promise<RunResult> {
  const t0 = performance.now()
  const proc = spawn(cmd, {
    cwd: opts.cwd,
    env: { ...process.env, ...opts.env },
    stdout: 'pipe',
    stderr: 'pipe',
  })

  const timer = opts.timeoutMs
    ? setTimeout(() => proc.kill('SIGKILL'), opts.timeoutMs)
    : null

  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ])
  if (timer) clearTimeout(timer)
  return { exitCode, durationMs: performance.now() - t0, stdout, stderr }
}
