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
});
