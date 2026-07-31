import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const fixture = join(root, 'fixtures/sample-repo');
const outDir = process.env.MCP_ARCHITECTURE_BENCHMARK_OUTPUT_DIR
  ? join(root, process.env.MCP_ARCHITECTURE_BENCHMARK_OUTPUT_DIR)
  : join(root, 'benchmark-artifacts');

function resolveCli(): string {
  for (const c of [
    process.env.ARCHITECTURE_READER_CLI_BIN,
    join(root, 'bin/native/architecture-reader-cli'),
    join(root, 'target/release/architecture-reader-cli'),
    join(root, 'target/debug/architecture-reader-cli'),
  ]) {
    if (c && existsSync(c)) return c;
  }
  throw new Error('architecture-reader-cli not built; run cargo build -p architecture-reader-cli');
}

function run(cli: string, tool: string, input: Record<string, unknown>) {
  const started = performance.now();
  const result = spawnSync(cli, [], {
    input: JSON.stringify({ tool, input }),
    encoding: 'utf8',
  });
  const wallMs = performance.now() - started;
  if (result.status !== 0) {
    throw new Error(result.stderr || `benchmark failed for ${tool}`);
  }
  const envelope = JSON.parse(result.stdout) as {
    status?: string;
    metrics?: { elapsedMs: number; nodeCount: number; edgeCount: number };
    answer?: Record<string, unknown>;
  };
  return { wallMs, envelope };
}

const cli = resolveCli();
const index = run(cli, 'architecture_index', { root: fixture, mode: 'full', useSynth: false });
const search = run(cli, 'architecture_search', { root: fixture, query: 'issue_token', limit: 5 });
const path = run(cli, 'architecture_path', {
  root: fixture,
  from: 'src/auth/token.ts',
  to: 'src/server/routes.ts',
});
const impact = run(cli, 'architecture_impact', {
  root: fixture,
  changedPaths: ['src/auth/token.ts'],
  maxDepth: 2,
});
const overview = run(cli, 'architecture_overview', { root: fixture, depth: 3 });

const report = {
  product: 'Spine',
  fixture,
  generatedAt: new Date().toISOString(),
  indexMs: index.wallMs,
  searchMs: search.wallMs,
  pathMs: path.wallMs,
  impactMs: impact.wallMs,
  overviewMs: overview.wallMs,
  nodes: index.envelope.metrics?.nodeCount,
  edges: index.envelope.metrics?.edgeCount,
  searchTopLabel: (search.envelope.answer as { matches?: { label?: string }[] })?.matches?.[0]?.label,
  impactIncoming: ((impact.envelope.answer as { incomingImpact?: unknown[] })?.incomingImpact ?? []).length,
  impactOutgoing: ((impact.envelope.answer as { outgoingImpact?: unknown[] })?.outgoingImpact ?? []).length,
  languages: (overview.envelope.answer as { languages?: unknown })?.languages,
  ok:
    index.envelope.status === 'ok' &&
    search.envelope.status === 'ok' &&
    impact.envelope.status === 'ok' &&
    overview.envelope.status === 'ok',
};

mkdirSync(outDir, { recursive: true });
const outPath = join(outDir, 'spine_public_proof.json');
writeFileSync(outPath, JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
if (!report.ok) process.exit(1);
