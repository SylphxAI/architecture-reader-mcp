import { readFileSync } from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'bun:test';
import { spawnSync } from 'node:child_process';

const repoRoot = path.resolve(import.meta.dirname, '../../..');

describe('check-no-ts-stdio-mcp gate', () => {
  it('script asserts Rust-only stdio authority and deleted TS adapter', () => {
    const script = readFileSync(path.join(repoRoot, 'scripts/check-no-ts-stdio-mcp.sh'), 'utf8');
    expect(script).toContain('resolve_rust_bin');
    expect(script).toContain('packages/mcp-server/src/index.ts must be deleted');
    expect(script).toContain('use_ts_transport');
    expect(script).toContain('rmcp::transport::stdio');
  });

  it('gate passes on current tree after TS adapter retirement', () => {
    const result = spawnSync('bash', ['scripts/check-no-ts-stdio-mcp.sh'], {
      cwd: repoRoot,
      encoding: 'utf8',
    });
    expect(result.status).toBe(0);
    expect(result.stdout).toContain('PASS');
  });

  it('deletion-ready gate passes on current tree', () => {
    const result = spawnSync('bash', ['scripts/check-ts-adapter-deletion-ready.sh'], {
      cwd: repoRoot,
      encoding: 'utf8',
    });
    expect(result.status).toBe(0);
    expect(result.stdout).toContain('PASS');
  });
});
