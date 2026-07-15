#!/usr/bin/env bash
# Post-deletion gate for transport/stdio-ts-adapter.
# Fails if TS stdio adapter files or opt-in routing remain after ts_deleted.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LEDGER="$ROOT/docs/specs/migration-ledger.json"
BIN="$ROOT/bin/architecture-reader-mcp"
TS_ENTRY="$ROOT/packages/mcp-server/src/index.ts"

require_ledger_state() {
  local capability="$1"
  local expected="$2"
  node - "$LEDGER" "$capability" "$expected" <<'NODE'
const [ledgerPath, capability, expected] = process.argv.slice(2);
const ledger = JSON.parse(require("node:fs").readFileSync(ledgerPath, "utf8"));
const entry = ledger.capabilities.find((cap) => cap.id === capability);
if (!entry) {
  console.error(`[check-ts-adapter-deleted] missing capability ${capability}`);
  process.exit(1);
}
if (entry.state !== expected) {
  console.error(
    `[check-ts-adapter-deleted] ${capability} is ${entry.state}; expected ${expected}`
  );
  process.exit(1);
}
NODE
}

echo "[check-ts-adapter-deleted] verifying transport/stdio-ts-adapter retirement in ${LEDGER}"

require_ledger_state "transport/stdio-ts-adapter" "ts_deleted"

if [[ -f "$TS_ENTRY" ]]; then
  echo "[check-ts-adapter-deleted] packages/mcp-server/src/index.ts must be deleted when transport/stdio-ts-adapter is ts_deleted" >&2
  exit 1
fi

if grep -q 'use_ts_transport' "$BIN"; then
  echo "[check-ts-adapter-deleted] bin must not retain TS stdio opt-in after ts_deleted" >&2
  exit 1
fi

if grep -q 'ARCHITECTURE_READER_MCP_TRANSPORT:-}" == "ts"' "$BIN"; then
  echo "[check-ts-adapter-deleted] bin must not route ARCHITECTURE_READER_MCP_TRANSPORT=ts after ts_deleted" >&2
  exit 1
fi

if grep -qE 'exec (bun|node)' "$BIN"; then
  echo "[check-ts-adapter-deleted] bin must not exec bun/node after ts_deleted" >&2
  exit 1
fi

echo "[check-ts-adapter-deleted] PASS — transport/stdio-ts-adapter retired; Rust rmcp is sole MCP transport"
