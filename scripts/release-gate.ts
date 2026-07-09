import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { invokeEngine } from '../packages/mcp-server/src/engine.ts';
import { runDoctor } from '../packages/mcp-server/src/doctor.ts';

const ARTIFACT_DIR_ENV = 'MCP_ARCHITECTURE_BENCHMARK_OUTPUT_DIR';
const DEFAULT_ARTIFACT_DIR = 'benchmark-artifacts';
const ARTIFACT_FILE = 'architecture_reader_release_gate.json';

type GateStatus = 'passed' | 'failed';

interface GateCheck {
  id: string;
  status: GateStatus;
  message: string;
  evidence?: Record<string, unknown>;
}

interface ReleaseGateReport {
  profile: 'architecture_reader_release_gate';
  generated_at: string;
  artifact_dir: string;
  status: GateStatus;
  summary: {
    total: number;
    passed: number;
    failed: number;
  };
  checks: GateCheck[];
}

const repoRoot = path.resolve(import.meta.dirname, '..');
const fixtureRoot = path.join(repoRoot, 'fixtures/sample-repo');

const addCheck = (
  checks: GateCheck[],
  id: string,
  passed: boolean,
  message: string,
  evidence?: Record<string, unknown>
): void => {
  checks.push({
    id,
    status: passed ? 'passed' : 'failed',
    message,
    ...(evidence ? { evidence } : {}),
  });
};

const fileExists = (relativePath: string): boolean =>
  existsSync(path.join(repoRoot, relativePath));

export async function buildReleaseGateReport(artifactDir: string): Promise<ReleaseGateReport> {
  const checks: GateCheck[] = [];
  const pkg = JSON.parse(readFileSync(path.join(repoRoot, 'package.json'), 'utf8')) as {
    version: string;
  };

  addCheck(
    checks,
    'rust:graph_core',
    fileExists('crates/architecture-reader-core/src/engine.rs'),
    'Rust architecture-reader-core graph engine is present'
  );

  addCheck(
    checks,
    'fixtures:sample_repo',
    fileExists('fixtures/sample-repo/package.json'),
    'Golden sample-repo fixture is checked in'
  );

  addCheck(
    checks,
    'spec:indexing_pipeline',
    fileExists('docs/specs/2026-07-09-indexing-pipeline.md'),
    'Indexing pipeline spec documents incremental refresh behavior'
  );

  const doctor = await runDoctor(pkg.version);
  addCheck(
    checks,
    'doctor:fixture_repo',
    doctor.checks.find((check) => check.id === 'fixture_repo')?.status === 'ok',
    'doctor reports the golden fixture repository is available',
    { doctorStatus: doctor.status }
  );

  const index = invokeEngine('architecture_index', { root: fixtureRoot, mode: 'full' });
  addCheck(
    checks,
    'boundary:architecture_index',
    index.status === 'ok' && (index.metrics?.nodeCount ?? 0) > 0,
    'architecture_index returns a populated graph envelope from the Rust CLI',
    { nodeCount: index.metrics?.nodeCount, edgeCount: index.metrics?.edgeCount }
  );

  const search = invokeEngine('architecture_search', {
    root: fixtureRoot,
    query: 'auth',
    limit: 5,
  });
  const matches = (search.answer as { matches?: unknown[] } | undefined)?.matches ?? [];
  addCheck(
    checks,
    'boundary:architecture_search',
    search.status === 'ok' && matches.length > 0,
    'architecture_search returns stable evidence locators for the fixture repo',
    { matchCount: matches.length }
  );

  const auto = invokeEngine('architecture_index', { root: fixtureRoot, mode: 'auto' });
  addCheck(
    checks,
    'boundary:incremental_cache_hit',
    auto.status === 'ok' &&
      (auto.answer as { refreshMode?: string } | undefined)?.refreshMode === 'cache_hit',
    'architecture_index mode=auto reuses the persisted index when file hashes are unchanged',
    { refreshMode: (auto.answer as { refreshMode?: string } | undefined)?.refreshMode }
  );

  const passed = checks.filter((check) => check.status === 'passed').length;
  const failed = checks.length - passed;

  return {
    profile: 'architecture_reader_release_gate',
    generated_at: new Date().toISOString(),
    artifact_dir: artifactDir,
    status: failed === 0 ? 'passed' : 'failed',
    summary: {
      total: checks.length,
      passed,
      failed,
    },
    checks,
  };
}

async function main(): Promise<void> {
  const artifactDir = path.resolve(
    process.env[ARTIFACT_DIR_ENV] ?? path.join(repoRoot, DEFAULT_ARTIFACT_DIR)
  );

  const report = await buildReleaseGateReport(artifactDir);
  mkdirSync(artifactDir, { recursive: true });
  const outputPath = path.join(artifactDir, ARTIFACT_FILE);
  writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(`Architecture reader release gate report written to ${outputPath}`);

  if (report.status !== 'passed') {
    for (const check of report.checks.filter((entry) => entry.status === 'failed')) {
      console.error(`[FAILED] ${check.id}: ${check.message}`);
    }
    process.exit(1);
  }
}

if (import.meta.main) {
  main().catch((error: unknown) => {
    console.error(error);
    process.exit(1);
  });
}