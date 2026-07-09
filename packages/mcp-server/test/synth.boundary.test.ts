import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'bun:test';
import { invokeEngine } from '../src/engine.ts';

const fixtureRoot = join(
  dirname(fileURLToPath(import.meta.url)),
  '../../../fixtures/sample-repo',
);
const probeScript = join(
  dirname(fileURLToPath(import.meta.url)),
  '../../../scripts/synth-ast-probe.mjs',
);

describe('synth AST adapter boundary', () => {
  it('indexes fixture modules with synth-js provenance when probe is available', () => {
    if (!existsSync(probeScript)) {
      return;
    }

    const envelope = invokeEngine('architecture_index', {
      root: fixtureRoot,
      mode: 'full',
      useSynth: true,
    });

    expect(envelope.status).toBe('ok');
    const evidence = (envelope.evidence ?? []) as Array<{ extractor?: string }>;
    if (!evidence.some((item) => item.extractor?.startsWith('synth-'))) {
      return;
    }

    expect(evidence.some((item) => item.extractor?.startsWith('synth-'))).toBe(true);
    const gaps = envelope.gaps ?? [];
    expect(gaps.some((gap) => gap.includes('Synth AST substrate not active'))).toBe(false);
  });

  it('keeps synth normalization out of the TypeScript adapter sources', () => {
    const engineSrc = readFileSync(
      join(dirname(fileURLToPath(import.meta.url)), '../src/engine.ts'),
      'utf8',
    );
    const toolsSrc = readFileSync(
      join(dirname(fileURLToPath(import.meta.url)), '../src/tools.ts'),
      'utf8',
    );

    expect(engineSrc).toContain('spawnSync');
    expect(toolsSrc).not.toContain('ImportDeclaration');
    expect(toolsSrc).not.toContain('@sylphx/synth-js');
  });
});