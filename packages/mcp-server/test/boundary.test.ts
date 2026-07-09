import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'bun:test';
import { invokeEngine } from '../src/engine.ts';

const fixtureRoot = join(
  dirname(fileURLToPath(import.meta.url)),
  '../../../fixtures/sample-repo',
);

describe('Rust core boundary', () => {
  it('delegates architecture_index to the Rust CLI and returns provenance envelope', () => {
    const envelope = invokeEngine('architecture_index', {
      root: fixtureRoot,
      mode: 'full',
    });

    expect(envelope.status).toBe('ok');
    expect(envelope.repository).toBeDefined();
    expect(envelope.metrics?.nodeCount).toBeGreaterThan(0);
    expect(Array.isArray(envelope.evidence)).toBe(true);
    expect((envelope.evidence ?? []).length).toBeGreaterThan(0);
  });

  it('keeps graph logic out of the TypeScript adapter sources', () => {
    const engineSrc = readFileSync(join(dirname(fileURLToPath(import.meta.url)), '../src/engine.ts'), 'utf8');
    const toolsSrc = readFileSync(join(dirname(fileURLToPath(import.meta.url)), '../src/tools.ts'), 'utf8');
    expect(engineSrc).toContain('spawnSync');
    expect(engineSrc).not.toContain('scan_repository');
    expect(toolsSrc).not.toMatch(/GraphNode|imports|walkdir/i);
  });
});