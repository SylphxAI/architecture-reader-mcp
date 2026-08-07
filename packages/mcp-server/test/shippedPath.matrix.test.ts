import { beforeAll, describe, expect, it } from 'bun:test';
import { chmodSync, existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { invokeEngine } from '../src/engine.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../../..');
const fixtureRoot = join(repoRoot, 'fixtures/sample-repo');

describe('shipped path matrix (Rust core, no legacy flags)', () => {
  let fakeNodeEnv: NodeJS.ProcessEnv;
  let nodeInvokeLog: string;

  beforeAll(() => {
    const releaseCli = join(repoRoot, 'target/release/architecture-reader-cli');
    const debugCli = join(repoRoot, 'target/debug/architecture-reader-cli');
    expect(existsSync(releaseCli) || existsSync(debugCli)).toBe(true);

    const probeDir = mkdtempSync(join(os.tmpdir(), 'architecture-reader-matrix-probe-'));
    nodeInvokeLog = join(probeDir, 'node-invoke.log');
    const fakeNode = join(probeDir, 'node');
    writeFileSync(
      fakeNode,
      `#!/usr/bin/env bash\nprintf '%s\\n' "$@" >> "${nodeInvokeLog}"\nexit 99\n`
    );
    chmodSync(fakeNode, 0o755);

    fakeNodeEnv = {
      ...process.env,
      ARCHITECTURE_READER_NODE: fakeNode,
      ARCHITECTURE_READER_ALLOW_LEGACY_ENGINE: '',
      ARCHITECTURE_READER_USE_SYNTH: '',
    };

    const index = invokeEngine(
      'architecture_index',
      { root: fixtureRoot, mode: 'full', useSynth: false },
      fakeNodeEnv
    );
    expect(index.status).toBe('ok');
  });

  const invoke = (tool: string, input: Record<string, unknown>) =>
    invokeEngine(tool, input, fakeNodeEnv);

  it('architecture_index returns populated graph envelope from Rust CLI', () => {
    const envelope = invoke('architecture_index', { root: fixtureRoot, mode: 'auto' });
    expect(envelope.status).toBe('ok');
    expect((envelope.metrics?.nodeCount ?? 0) > 0).toBe(true);
    expect(existsSync(nodeInvokeLog)).toBe(false);
  });

  it('architecture_status reports indexed repository state', () => {
    const envelope = invoke('architecture_status', { root: fixtureRoot });
    expect(envelope.status).toBe('ok');
    expect(envelope.repository).toBeDefined();
    expect(existsSync(nodeInvokeLog)).toBe(false);
  });

  it('architecture_status reports honest synth-off default coverage', () => {
    const envelope = invoke('architecture_status', { root: fixtureRoot });
    const coverage = (envelope.answer as {
      coverage?: { synthMode?: string; importGraphRoute?: string };
    } | undefined)?.coverage;
    expect(coverage?.synthMode).toBe('off');
    expect(coverage?.importGraphRoute).toBe('regex_fallback');
    expect(
      (envelope.gaps ?? []).some((gap) =>
        String(gap).includes('importGraphRoute=regex_fallback')
      )
    ).toBe(true);
    expect(existsSync(nodeInvokeLog)).toBe(false);
  });

  it('architecture_overview returns package and module slices', () => {
    const envelope = invoke('architecture_overview', { root: fixtureRoot, depth: 2 });
    expect(envelope.status).toBe('ok');
    const answer = envelope.answer as { packages?: unknown[]; modules?: unknown[] } | undefined;
    expect((answer?.packages?.length ?? 0) > 0).toBe(true);
    expect(existsSync(nodeInvokeLog)).toBe(false);
  });

  it('architecture_search returns auth matches with evidence', () => {
    const envelope = invoke('architecture_search', {
      root: fixtureRoot,
      query: 'auth',
      limit: 5,
    });
    expect(envelope.status).toBe('ok');
    const matches = (envelope.answer as { matches?: unknown[] } | undefined)?.matches ?? [];
    expect(matches.length).toBeGreaterThan(0);
    expect(existsSync(nodeInvokeLog)).toBe(false);
  });

  it('architecture_trace follows symbol call edges in the fixture repo', () => {
    const envelope = invoke('architecture_trace', {
      root: fixtureRoot,
      from: 'authMiddleware',
      to: 'validateToken',
      relation: 'calls',
      maxDepth: 4,
    });
    expect(envelope.status).toBe('ok');
    const tracePath = (envelope.answer as { path?: unknown[] } | undefined)?.path ?? [];
    expect(tracePath.length).toBeGreaterThanOrEqual(2);
    expect(existsSync(nodeInvokeLog)).toBe(false);
  });

  it('architecture_impact reports direct impact for changed paths', () => {
    const envelope = invoke('architecture_impact', {
      root: fixtureRoot,
      changedPaths: ['src/server/routes.ts'],
    });
    expect(envelope.status).toBe('ok');
    const directImpact =
      (envelope.answer as { directImpact?: unknown[] } | undefined)?.directImpact ?? [];
    expect(directImpact.length).toBeGreaterThan(0);
    expect(existsSync(nodeInvokeLog)).toBe(false);
  });

  it('default bin resolves staged rmcp server', () => {
    const bin = join(repoRoot, 'bin/spine');
    expect(existsSync(bin)).toBe(true);
    const staged = join(repoRoot, 'bin/native/spine-mcp-server');
    const release = join(repoRoot, 'target/release/spine-mcp-server');
    expect(existsSync(staged) || existsSync(release)).toBe(true);
  });

  it('architecture_evidence resolves ids from the indexed graph', () => {
    const search = invoke('architecture_search', {
      root: fixtureRoot,
      query: 'auth',
      limit: 1,
      includeEvidence: true,
    });
    const evidence = search.evidence ?? [];
    const evidenceId =
      (evidence[0] as { id?: string } | undefined)?.id ??
      ((search.answer as { matches?: Array<{ id?: string }> } | undefined)?.matches?.[0]?.id);

    expect(evidenceId).toBeDefined();

    const envelope = invoke('architecture_evidence', {
      root: fixtureRoot,
      ids: [evidenceId as string],
    });
    expect(envelope.status).toBe('ok');
    expect((envelope.evidence ?? []).length).toBeGreaterThan(0);
    expect(existsSync(nodeInvokeLog)).toBe(false);
  });
});