#!/usr/bin/env bun
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dir, '..');
const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8')) as {
  name?: string;
  version?: string;
  bin?: Record<string, string>;
};
const server = existsSync(join(root, 'server.json'))
  ? (JSON.parse(readFileSync(join(root, 'server.json'), 'utf8')) as { title?: string })
  : {};
const plan = {
  canonicalName: '@sylphx/spine',
  actualName: pkg.name,
  version: pkg.version,
  brandBin: pkg.bin?.spine,
  marketplaceTitle: server.title,
  brandSole: pkg.name === '@sylphx/spine' && Boolean(pkg.bin?.spine),
  ok: false as boolean,
};
plan.ok = Boolean(plan.brandSole && server.title === 'Spine' && plan.version);
console.log(JSON.stringify(plan, null, 2));
process.exit(plan.ok ? 0 : 1);
