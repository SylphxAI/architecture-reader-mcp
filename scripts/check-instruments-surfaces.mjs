#!/usr/bin/env node
/**
 * Sylphx Instruments surface gate — verifies brand repos expose Core/SDK/CLI/MCP markers.
 * Exit 0 only if all required predicates hold for the six Instruments products.
 */
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SYLPHX = join(__dirname, '../../..'); // .../SylphxAI when script is in architecture-reader-mcp/scripts
// architecture-reader-mcp/scripts -> architecture-reader-mcp -> SylphxAI
const ORG = join(__dirname, '../..');

const products = [
  {
    brand: 'Citra',
    dir: 'pdf-reader-mcp',
    sdk: ['src/sdk.ts'],
    binKeys: ['citra', 'pdf-reader-mcp'],
    exportKeys: ['./sdk', './citra'],
    mcpMarkers: ['src/runtime-entry.ts', 'src/mcp.ts', 'dist/runtime-entry.js'],
    coreMarkers: ['crates/pdf-reader-core', 'src/pure-rust.ts'],
  },
  {
    brand: 'Iris',
    dir: 'image-reader-mcp',
    sdk: ['src/sdk.ts'],
    binKeys: ['iris', 'image-reader-mcp'],
    exportKeys: ['./sdk'],
    mcpMarkers: ['bin/image-reader-mcp', 'src/mcp.ts'],
    coreMarkers: ['crates', 'src/handlers/readImage.ts'],
  },
  {
    brand: 'Cue',
    dir: 'video-reader-mcp',
    sdk: ['src/sdk.ts'],
    binKeys: ['cue', 'video-reader-mcp'],
    exportKeys: ['./sdk'],
    mcpMarkers: ['bin/video-reader-mcp', 'src/mcp.ts'],
    coreMarkers: ['crates', 'src/handlers/readVideo.ts'],
  },
  {
    brand: 'Prism',
    dir: 'smart-reader-mcp',
    sdk: ['src/sdk.ts'],
    binKeys: ['prism', 'smart-reader-mcp'],
    exportKeys: ['./sdk'],
    mcpMarkers: ['bin/smart-reader-mcp', 'src/mcp.ts'],
    coreMarkers: ['crates', 'src/handlers/readMedia.ts'],
  },
  {
    brand: 'Spine',
    dir: 'architecture-reader-mcp',
    sdk: ['packages/mcp-server/src/spine-sdk.ts'],
    binKeys: ['spine', 'architecture-reader-mcp'],
    exportKeys: ['./sdk', './spine'],
    packageJson: 'packages/mcp-server/package.json',
    mcpMarkers: ['bin/architecture-reader-mcp', 'crates/architecture-reader-mcp-server'],
    coreMarkers: ['crates/architecture-reader-core', 'bin/spine'],
  },
  {
    brand: 'Lookout',
    dir: 'lookout',
    sdk: ['src/sdk.ts'],
    binKeys: ['lookout'],
    exportKeys: ['./sdk', '.'],
    mcpMarkers: ['src/mcp.ts', 'server.json'],
    coreMarkers: ['src/engine.ts', 'src/ssrf.ts', 'crates/lookout-core'],
  },
];

function anyExists(root, rels) {
  return rels.some((r) => existsSync(join(root, r)));
}

function loadPkg(root, rel = 'package.json') {
  const p = join(root, rel);
  return JSON.parse(readFileSync(p, 'utf8'));
}

const rows = [];
let failed = 0;
for (const p of products) {
  const root = join(ORG, p.dir);
  const issues = [];
  if (!existsSync(root)) {
    issues.push('repo checkout missing');
    failed += 1;
    rows.push({ brand: p.brand, ok: false, issues });
    continue;
  }
  const pkgRel = p.packageJson || 'package.json';
  let pkg;
  try {
    pkg = loadPkg(root, pkgRel);
  } catch (e) {
    issues.push(`package.json unreadable: ${pkgRel}`);
  }
  if (pkg) {
    const bins = pkg.bin || {};
    for (const k of p.binKeys) {
      if (!bins[k]) issues.push(`missing bin.${k}`);
    }
    const exports = pkg.exports || {};
    for (const k of p.exportKeys) {
      if (typeof exports === 'object' && exports && !(k in exports) && k !== '.') {
        // allow '.' only required if listed and exports is object
        if (k !== '.' || !exports['.']) {
          if (!(k in exports)) issues.push(`missing exports['${k}']`);
        }
      }
    }
  }
  if (!p.sdk.every((s) => existsSync(join(root, s)))) {
    issues.push('missing SDK source');
  }
  if (!anyExists(root, p.mcpMarkers)) issues.push('missing MCP markers');
  if (!anyExists(root, p.coreMarkers)) issues.push('missing core markers');
  // tests marker: any test dir
  const hasTests =
    existsSync(join(root, 'test')) ||
    existsSync(join(root, 'tests')) ||
    existsSync(join(root, 'packages/mcp-server/test')) ||
    existsSync(join(root, 'crates'));
  if (!hasTests) issues.push('no tests directory observed');

  const ok = issues.length === 0;
  if (!ok) failed += 1;
  rows.push({ brand: p.brand, ok, issues, root });
}

console.log('Sylphx Instruments surface gate\n');
for (const r of rows) {
  const mark = r.ok ? 'PASS' : 'FAIL';
  console.log(`${mark}  ${r.brand}`);
  if (!r.ok) for (const i of r.issues) console.log(`       - ${i}`);
}
console.log(`\n${rows.length - failed}/${rows.length} products PASS`);
process.exit(failed ? 1 : 0);
