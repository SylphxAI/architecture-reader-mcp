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
    process.env.ARCHITECTURE_READER_CLI,
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
const search = run(cli, 'architecture_search', {
  root: fixture,
  query: 'issue_token',
  limit: 5,
  includeNeighbors: true,
});
const openapiSearch = run(cli, 'architecture_search', {
  root: fixture,
  query: '/v1/tokens',
  limit: 5,
  types: ['route', 'symbol'],
});
const path = run(cli, 'architecture_path', {
  root: fixture,
  from: 'src/auth/token.ts',
  to: 'src/server/routes.ts',
});
const impact = run(cli, 'architecture_impact', {
  root: fixture,
  changedPaths: ['src/auth/token.ts', 'does/not/exist.ts'],
  maxDepth: 2,
});
const overview = run(cli, 'architecture_overview', {
  root: fixture,
  focus: 'src/auth/token.ts',
  depth: 3,
});
const status = run(cli, 'architecture_status', { root: fixture });
const pack = run(cli, 'architecture_context_pack', {
  root: fixture,
  focus: 'src/auth/token.ts',
  maxNeighbors: 8,
});
const browse = run(cli, 'architecture_search', {
  root: fixture,
  query: '',
  limit: 8,
});
const matchId = (search.envelope.answer as { matches?: { id?: string }[] })?.matches?.[0]?.id;
const evidence = matchId
  ? run(cli, 'architecture_evidence', { root: fixture, ids: [matchId, 'ev_missing'] })
  : null;

const answer = <T,>(env: { answer?: Record<string, unknown> }, key: string): T | undefined =>
  env.answer?.[key] as T | undefined;

const extractors = answer<string[]>(index.envelope, 'extractors')
  ?? answer<string[]>(status.envelope, 'extractors')
  ?? [];

const report = {
  product: 'Spine',
  fixture,
  generatedAt: new Date().toISOString(),
  indexMs: index.wallMs,
  searchMs: search.wallMs,
  pathMs: path.wallMs,
  impactMs: impact.wallMs,
  overviewMs: overview.wallMs,
  statusMs: status.wallMs,
  packMs: pack.wallMs,
  browseMs: browse.wallMs,
  evidenceMs: evidence?.wallMs,
  nodes: index.envelope.metrics?.nodeCount,
  edges: index.envelope.metrics?.edgeCount,
  searchTopLabel: (search.envelope.answer as { matches?: { label?: string }[] })?.matches?.[0]?.label,
  searchHasNeighbors: Boolean(
    (search.envelope.answer as { matches?: { neighbors?: unknown[] }[] })?.matches?.[0]?.neighbors,
  ),
  openapiRouteHit: Boolean(
    ((openapiSearch.envelope.answer as { matches?: { label?: string; kind?: string }[] })?.matches ?? []).some(
      (m) => (m.label ?? '').includes('/v1/tokens') || m.kind === 'route',
    ),
  ),
  pathHopCount: (path.envelope.answer as { hopCount?: number })?.hopCount,
  pathHasMermaid: String((path.envelope.answer as { mermaid?: string })?.mermaid ?? '').includes('graph LR'),
  impactIncoming: ((impact.envelope.answer as { incomingImpact?: unknown[] })?.incomingImpact ?? []).length,
  impactOutgoing: ((impact.envelope.answer as { outgoingImpact?: unknown[] })?.outgoingImpact ?? []).length,
  impactUnknown: ((impact.envelope.answer as { unknownImpact?: unknown[] })?.unknownImpact ?? []).length,
  impactHasMermaid: Boolean((impact.envelope.answer as { mermaid?: { incoming?: string } })?.mermaid?.incoming?.includes('graph LR')),
  hasTopFanIn: Array.isArray((overview.envelope.answer as { topFanIn?: unknown[] })?.topFanIn),
  hasTopFanOut: Array.isArray((overview.envelope.answer as { topFanOut?: unknown[] })?.topFanOut),
  hasCycles: Array.isArray((overview.envelope.answer as { cycles?: unknown[] })?.cycles),
  hasOrphans: Array.isArray((overview.envelope.answer as { orphans?: unknown[] })?.orphans)
    || Array.isArray((status.envelope.answer as { orphans?: unknown[] })?.orphans),
  overviewHasMermaid:
    (overview.envelope.answer as { mermaid?: string | null })?.mermaid == null
    || String((overview.envelope.answer as { mermaid?: string })?.mermaid ?? '').includes('graph LR'),
  packHasMermaid: String((pack.envelope.answer as { mermaid?: string })?.mermaid ?? '').includes('graph LR'),
  browseNonEmpty: ((browse.envelope.answer as { matches?: unknown[] })?.matches ?? []).length > 0,
  statusLanguages: (status.envelope.answer as { languages?: unknown })?.languages,
  evidenceMissing: ((evidence?.envelope.answer as { missing?: unknown[] })?.missing ?? []).length,
  languages: (overview.envelope.answer as { languages?: unknown })?.languages,
  extractorsSample: Array.isArray(extractors) ? extractors.slice(0, 24) : extractors,
  multiFormat: {
    hasOpenapiFixture: existsSync(join(fixture, 'openapi.yaml')),
    hasProtoFixture: existsSync(join(fixture, 'proto/auth.proto')),
    hasGraphqlFixture: existsSync(join(fixture, 'graphql/schema.graphql')),
    hasSqlFixture: existsSync(join(fixture, 'db/schema.sql')),
    hasDockerfile: existsSync(join(fixture, 'Dockerfile')),
    hasMakefile: existsSync(join(fixture, 'Makefile')),
    hasCodeowners: existsSync(join(fixture, 'CODEOWNERS')),
    hasHelmChart: existsSync(join(fixture, 'charts/demo/Chart.yaml')),
    hasK8s: existsSync(join(fixture, 'k8s/deploy.yaml')),
    hasTerraform: existsSync(join(fixture, 'infra/main.tf')),
  },
  ok:
    index.envelope.status === 'ok' &&
    search.envelope.status === 'ok' &&
    impact.envelope.status === 'ok' &&
    overview.envelope.status === 'ok' &&
    status.envelope.status === 'ok' &&
    pack.envelope.status === 'ok' &&
    browse.envelope.status === 'ok' &&
    ((impact.envelope.answer as { unknownImpact?: unknown[] })?.unknownImpact ?? []).length >= 1 &&
    String((path.envelope.answer as { mermaid?: string })?.mermaid ?? '').includes('graph LR'),
};

mkdirSync(outDir, { recursive: true });
const outPath = join(outDir, 'spine_public_proof.json');
writeFileSync(outPath, JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
if (!report.ok) process.exit(1);
