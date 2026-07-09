import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(fileURLToPath(import.meta.url));
const fixture = join(root, '../fixtures/sample-repo');
const cli = join(root, '../target/release/architecture-reader-cli');

function run(tool: string, input: Record<string, unknown>) {
  const started = performance.now();
  const result = spawnSync(cli, [], {
    input: JSON.stringify({ tool, input }),
    encoding: 'utf8',
  });
  const elapsedMs = performance.now() - started;
  if (result.status !== 0) {
    throw new Error(result.stderr || `benchmark failed for ${tool}`);
  }
  const envelope = JSON.parse(result.stdout) as {
    metrics?: { elapsedMs: number; nodeCount: number; edgeCount: number };
  };
  return { elapsedMs, engineMs: envelope.metrics?.elapsedMs ?? 0, envelope };
}

const index = run('architecture_index', { root: fixture, mode: 'full' });
const search = run('architecture_search', { root: fixture, query: 'auth', limit: 5 });

console.log(
  JSON.stringify(
    {
      fixture,
      indexMs: index.elapsedMs,
      searchMs: search.elapsedMs,
      nodes: index.envelope.metrics?.nodeCount,
      edges: index.envelope.metrics?.edgeCount,
    },
    null,
    2,
  ),
);