import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

export function resolveCliBinary(): string {
  const env = process.env['ARCHITECTURE_READER_CLI'];
  if (env && existsSync(env)) return env;

  const release = join(here, '../../../target/release/architecture-reader-cli');
  if (existsSync(release)) return release;

  const debug = join(here, '../../../target/debug/architecture-reader-cli');
  if (existsSync(debug)) return debug;

  return 'architecture-reader-cli';
}

export type ToolEnvelope = {
  status: string;
  repository?: unknown;
  answer?: unknown;
  evidence?: unknown[];
  gaps?: string[];
  metrics?: { elapsedMs: number; nodeCount: number; edgeCount: number };
  code?: string;
  message?: string;
  nextAction?: string;
};

export function invokeEngine(
  tool: string,
  input: Record<string, unknown>,
  env: NodeJS.ProcessEnv = process.env,
): ToolEnvelope {
  const binary = resolveCliBinary();
  const payload = JSON.stringify({ tool, input });
  const result = spawnSync(binary, [], {
    input: payload,
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    env,
  });

  if (result.error) {
    return {
      status: 'error',
      code: 'INTERNAL_ERROR',
      message: `Failed to launch architecture reader engine: ${result.error.message}`,
      nextAction: 'Build the Rust CLI with `cargo build --release`.',
    };
  }

  if (result.status !== 0) {
    return {
      status: 'error',
      code: 'INTERNAL_ERROR',
      message: result.stderr || `Engine exited with status ${result.status}`,
    };
  }

  return JSON.parse(result.stdout) as ToolEnvelope;
}