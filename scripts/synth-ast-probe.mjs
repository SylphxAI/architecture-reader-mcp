#!/usr/bin/env bun
import { readFileSync } from 'node:fs';

const request = JSON.parse(await Bun.stdin.text());
const filePath = request.path;

if (typeof filePath !== 'string' || filePath.length === 0) {
  console.error(JSON.stringify({ status: 'error', message: 'path is required' }));
  process.exit(1);
}

const source = readFileSync(filePath, 'utf8');
const { parse } = await import('@sylphx/synth-js');
const typescript = filePath.endsWith('.ts') || filePath.endsWith('.tsx');
const tree = parse(source, { typescript, sourceType: 'module' });
process.stdout.write(`${JSON.stringify(tree)}\n`);