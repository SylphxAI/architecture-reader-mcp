#!/usr/bin/env node
/**
 * Run lightweight Instruments verification across sibling checkouts.
 * Does not replace full product CI — proves surface gate + key unit suites.
 */
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ORG = join(dirname(fileURLToPath(import.meta.url)), '../..');

function run(cwd, cmd, args) {
  console.log(`\n>> (${cwd}) ${cmd} ${args.join(' ')}`);
  const r = spawnSync(cmd, args, { cwd, stdio: 'inherit', env: process.env });
  if (r.status !== 0) {
    console.error(`FAILED in ${cwd}`);
    process.exit(r.status ?? 1);
  }
}

// 1) surface gate
run(join(ORG, 'architecture-reader-mcp'), 'node', [
  'scripts/check-instruments-surfaces.mjs',
]);

// 2) Lookout offline tests
run(join(ORG, 'lookout'), 'bun', ['test']);

// 3) Contract tests for brand SDKs
for (const repo of ['pdf-reader-mcp', 'image-reader-mcp', 'video-reader-mcp', 'smart-reader-mcp']) {
  const cwd = join(ORG, repo);
  if (!existsSync(cwd)) continue;
  if (repo === 'pdf-reader-mcp') {
    run(cwd, 'bun', ['test', 'test/citra-sdk-export.test.ts']);
  } else {
    run(cwd, 'bun', ['test', 'test/instruments-sdk-contract.test.ts']);
  }
}

// 4) Spine core path test
run(join(ORG, 'architecture-reader-mcp'), 'cargo', [
  'test',
  '-p',
  'architecture-reader-core',
  'path_returns_hops',
  '--',
  '--nocapture',
]);

// 5) Spine SDK unit
const mcpPkg = join(ORG, 'architecture-reader-mcp/packages/mcp-server');
if (existsSync(join(mcpPkg, 'node_modules'))) {
  run(mcpPkg, 'bun', ['test', 'test/spine-sdk.test.ts']);
} else {
  console.log('\n>> skip spine-sdk test (no node_modules); run bun install in packages/mcp-server');
}

console.log('\nInstruments lightweight verification PASS');
