#!/usr/bin/env bun
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dir, '..');
const pkgPath = join(root, 'packages/mcp-server/package.json');
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8')) as {
  name?: string;
  version?: string;
  bin?: Record<string, string>;
};
const server = JSON.parse(readFileSync(join(root, 'server.json'), 'utf8')) as { title?: string };
const plan = {
  repoRoot: root,
  publishPackageDir: 'packages/mcp-server',
  transitionalName: pkg.name,
  version: pkg.version,
  brandBins: Object.keys(pkg.bin ?? {}),
  marketplaceTitle: server.title,
  brandPublishDoc: existsSync(join(root, 'docs/BRAND_PUBLISH.md')),
  skill: existsSync(join(root, 'skills/spine/SKILL.md')),
  targetBrandName: '@sylphx/spine',
  npmAuthRequiredForLivePublish: true,
  ok: pkg.name === '@sylphx/architecture-reader-mcp' && server.title === 'Spine',
};
console.log(JSON.stringify(plan, null, 2));
process.exit(plan.ok ? 0 : 1);
