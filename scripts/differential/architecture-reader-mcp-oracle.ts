#!/usr/bin/env bun
/**
 * TS pure-contract oracle for architecture-reader-mcp differential residual (rej-010 / BW2).
 *
 * Pure residual only:
 * - tool route contract / allow-list / server contract
 *
 * Fail-closed: no SKIP-as-pass. Does NOT claim architecture_* graph effect parity,
 * HTTP transport, parity_proven, or authority_rust.
 */
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const CORPUS_PATH = join(__dirname, "fixtures/architecture-reader-mcp-corpus.json");

interface ToolRouteCase {
  id: string;
  tool: string;
  expect: string | null;
}

interface Corpus {
  corpusVersion: number;
  toolRouteCases: ToolRouteCase[];
  serverContract: { name: string; version: string; tools: string[] };
  allowList: { tools: string[] };
}

export interface DifferentialCase {
  readonly id: string;
  readonly slice: string;
  readonly domain: "toolRouteContract" | "serverContract" | "allowList";
  readonly input: Record<string, unknown>;
  readonly output: unknown;
}

function fixtureCorpusHash(raw: string): string {
  return createHash("sha256").update(raw).digest("hex");
}

function main(): void {
  const raw = readFileSync(CORPUS_PATH, "utf8");
  const corpus = JSON.parse(raw) as Corpus;
  const cases: DifferentialCase[] = [];

  for (const c of corpus.toolRouteCases) {
    cases.push({
      id: c.id,
      slice: "tool-route-contract",
      domain: "toolRouteContract",
      input: { tool: c.tool },
      output: { route: c.expect },
    });
  }

  cases.push({
    id: "server-contract",
    slice: "server-contract",
    domain: "serverContract",
    input: {},
    output: corpus.serverContract,
  });

  cases.push({
    id: "allow-list",
    slice: "allow-list",
    domain: "allowList",
    input: {},
    output: { tools: corpus.allowList.tools },
  });

  const payload = {
    corpusVersion: corpus.corpusVersion,
    fixtureCorpusHash: fixtureCorpusHash(raw),
    cases,
  };
  process.stdout.write(JSON.stringify(payload) + "\n");
}

main();
