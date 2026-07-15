import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'bun:test';

const repoRoot = path.resolve(import.meta.dirname, '../../..');

describe('TS stdio adapter deletion matrix (adversarial admission)', () => {
  it('npm bin routes exclusively to Rust rmcp', () => {
    const bin = readFileSync(path.join(repoRoot, 'bin/architecture-reader-mcp'), 'utf8');
    expect(bin).toContain('resolve_rust_bin');
    expect(bin).toContain('architecture-reader-mcp-server');
    expect(bin).not.toContain('use_ts_transport');
    expect(bin).not.toContain('ARCHITECTURE_READER_MCP_TRANSPORT:-}" == "ts"');
    expect(bin).not.toContain('exec bun');
    expect(bin).not.toContain('exec node');
    expect(bin).not.toContain('packages/mcp-server/src/index.ts');
  });

  it('TS stdio adapter source is deleted', () => {
    expect(existsSync(path.join(repoRoot, 'packages/mcp-server/src/index.ts'))).toBe(false);
  });

  it('deletion gate script enforces ts_deleted ledger state', () => {
    const script = readFileSync(
      path.join(repoRoot, 'scripts/check-ts-adapter-deletion-ready.sh'),
      'utf8'
    );
    expect(script).toContain('require_ledger_state "transport/stdio-ts-adapter" "ts_deleted"');
    expect(script).toContain('packages/mcp-server/src/index.ts must be deleted');
    expect(script).toContain('use_ts_transport');
  });

  it('no-ts-stdio gate blocks reintroduction of TS adapter', () => {
    const script = readFileSync(path.join(repoRoot, 'scripts/check-no-ts-stdio-mcp.sh'), 'utf8');
    expect(script).toContain('packages/mcp-server/src/index.ts must be deleted');
    expect(script).toContain('use_ts_transport');
    expect(script).toContain('rmcp::transport::stdio');
  });

  it('ledger records stdio-ts-adapter as ts_deleted', () => {
    const ledger = JSON.parse(
      readFileSync(path.join(repoRoot, 'docs/specs/migration-ledger.json'), 'utf8')
    ) as {
      capabilities: Array<{ id: string; state: string }>;
      summary: { ts_deleted: number; ts_only: number; completion_progress: number };
    };
    const tsAdapter = ledger.capabilities.find((cap) => cap.id === 'transport/stdio-ts-adapter');
    expect(tsAdapter?.state).toBe('ts_deleted');
    expect(ledger.summary.ts_deleted).toBe(10);
    expect(ledger.summary.ts_only).toBe(0);
    expect(ledger.summary.completion_progress).toBe(1.0);
  });

  it('ledger records all in-scope capabilities as ts_deleted', () => {
    const ledger = JSON.parse(
      readFileSync(path.join(repoRoot, 'docs/specs/migration-ledger.json'), 'utf8')
    ) as {
      capabilities: Array<{ id: string; state: string }>;
      summary: { ts_deleted: number; completion_progress: number; authority_progress: number };
    };
    for (const cap of ledger.capabilities) {
      expect(cap.state).toBe('ts_deleted');
    }
    expect(ledger.summary.ts_deleted).toBe(10);
    expect(ledger.summary.completion_progress).toBe(1.0);
    expect(ledger.summary.authority_progress).toBe(1.0);
  });

  it('install doctor remains available without TS MCP adapter', () => {
    expect(existsSync(path.join(repoRoot, 'packages/mcp-server/src/doctor.ts'))).toBe(true);
    const pkg = JSON.parse(
      readFileSync(path.join(repoRoot, 'package.json'), 'utf8')
    ) as { scripts: Record<string, string> };
    expect(pkg.scripts.doctor).toContain('doctor.ts');
    expect(pkg.scripts.doctor).not.toContain('index.ts');
  });
});
