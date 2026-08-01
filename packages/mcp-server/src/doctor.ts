import { existsSync, readFileSync } from 'node:fs';
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
  const primary = [
    'architecture_index',
    'architecture_status',
    'architecture_overview',
    'architecture_search',
    'architecture_path',
    'architecture_impact',
  ];
  const advanced = [
    'architecture_trace',
    'architecture_evidence',
    'architecture_context_pack',
  ];
  const required = [...primary, ...advanced];

  const routesPath = join(here, '../../../crates/architecture-reader-mcp-server/src/tool_routes.rs');
  const enginePath = join(here, '../../../crates/architecture-reader-core/src/engine.rs');
  const toolsPath = join(here, 'tools.ts');
  const spineSdkPath = join(here, 'spine-sdk.ts');

  if (!existsSync(routesPath) || !existsSync(enginePath)) {
    return {
      id: 'tool_surface',
      status: 'fail',
      message: 'Missing Rust tool_routes/engine sources for architecture tools.',
    };
  }

  const routes = readFileSync(routesPath, 'utf8');
  const engine = readFileSync(enginePath, 'utf8');
  const tools = existsSync(toolsPath) ? readFileSync(toolsPath, 'utf8') : '';
  const sdk = existsSync(spineSdkPath) ? readFileSync(spineSdkPath, 'utf8') : '';

  const missing: string[] = [];
  for (const name of required) {
    if (!routes.includes(`"${name}"`) && !routes.includes(name)) {
      missing.push(`${name}@routes`);
    }
    if (!engine.includes(`"${name}"`)) {
      missing.push(`${name}@engine`);
    }
  }
  // TS adapter should expose path schema + SDK path method for product surface
  if (!tools.includes('architecturePathSchema') && !tools.includes('architecture_path')) {
    missing.push('architecturePathSchema@tools.ts');
  }
  if (!sdk.includes('path(') && !sdk.includes('.path')) {
    missing.push('Spine.path@sdk');
  }

  if (missing.length > 0) {
    return {
      id: 'tool_surface',
      status: 'fail',
      message: `Missing tool surface markers: ${missing.join(', ')}`,
    };
  }

  return {
    id: 'tool_surface',
    status: 'ok',
    message: `MCP/Rust/SDK expose ${required.length} architecture tools (including architecture_path).`,
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
  const pkg = await import('../package.json', { with: { type: 'json' } });
  const report = await runDoctor(pkg.default.version);
  console.log(JSON.stringify(report, null, 2));
  process.exit(report.status === 'unavailable' ? 1 : 0);
}