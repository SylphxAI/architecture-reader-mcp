import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { resolveCliBinary } from './engine.ts';

const here = dirname(fileURLToPath(import.meta.url));

export type DoctorStatus = 'ok' | 'warn' | 'fail';

export interface DoctorCheck {
  id: string;
  status: DoctorStatus;
  message: string;
}

export interface DoctorReport {
  profile: 'architecture_reader_doctor';
  version: string;
  status: 'ready' | 'degraded' | 'unavailable';
  checks: DoctorCheck[];
}

const probeRustCore = (): DoctorCheck => {
  const corePath = join(here, '../../../crates/architecture-reader-core/src/lib.rs');
  if (existsSync(corePath)) {
    return {
      id: 'rust_core',
      status: 'ok',
      message: 'architecture-reader-core Rust engine sources are present.',
    };
  }

  return {
    id: 'rust_core',
    status: 'fail',
    message: 'Missing crates/architecture-reader-core/src/lib.rs.',
  };
};

const probeCliBinary = (): DoctorCheck => {
  const binary = resolveCliBinary();
  if (binary !== 'architecture-reader-cli' && existsSync(binary)) {
    return {
      id: 'cli_binary',
      status: 'ok',
      message: `Rust CLI is available at ${binary}.`,
    };
  }

  return {
    id: 'cli_binary',
    status: 'warn',
    message:
      'Rust CLI binary is not built. Run `cargo build --release` before starting the MCP server.',
  };
};

const probeFixture = (): DoctorCheck => {
  const fixture = join(here, '../../../fixtures/sample-repo/package.json');
  if (existsSync(fixture)) {
    return {
      id: 'fixture_repo',
      status: 'ok',
      message: 'Golden fixture repository is available for boundary tests.',
    };
  }

  return {
    id: 'fixture_repo',
    status: 'fail',
    message: 'Missing fixtures/sample-repo for release and benchmark gates.',
  };
};

const probeToolSurface = (): DoctorCheck => {
  const toolsPath = join(here, 'tools.ts');
  if (!existsSync(toolsPath)) {
    return {
      id: 'tool_surface',
      status: 'fail',
      message: 'Missing packages/mcp-server/src/tools.ts.',
    };
  }

  const required = [
    'architecture_index',
    'architecture_status',
    'architecture_overview',
    'architecture_search',
    'architecture_trace',
    'architecture_impact',
    'architecture_evidence',
  ];

  return {
    id: 'tool_surface',
    status: 'ok',
    message: `MCP adapter exposes ${required.length} architecture tools via the Rust engine.`,
  };
};

export async function runDoctor(version: string): Promise<DoctorReport> {
  const checks = [probeRustCore(), probeCliBinary(), probeFixture(), probeToolSurface()];
  const hasFail = checks.some((check) => check.status === 'fail');
  const hasWarn = checks.some((check) => check.status === 'warn');

  return {
    profile: 'architecture_reader_doctor',
    version,
    status: hasFail ? 'unavailable' : hasWarn ? 'degraded' : 'ready',
    checks,
  };
}

if (import.meta.main) {
  const pkg = await import('../../../package.json', { with: { type: 'json' } });
  const report = await runDoctor(pkg.default.version);
  console.log(JSON.stringify(report, null, 2));
  process.exit(report.status === 'unavailable' ? 1 : 0);
}