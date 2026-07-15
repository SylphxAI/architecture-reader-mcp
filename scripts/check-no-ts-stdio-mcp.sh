#!/usr/bin/env bash
# S3 gate: default MCP stdio transport must delegate solely to Rust rmcp.
# TS stdio adapter is retired (transport/stdio-ts-adapter → ts_deleted).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${ROOT}/bin/architecture-reader-mcp"
RUST_MAIN="${ROOT}/crates/architecture-reader-mcp-server/src/main.rs"
TS_ENTRY="${ROOT}/packages/mcp-server/src/index.ts"
TS_ADAPTER_GATE="${ROOT}/scripts/check-ts-adapter-deletion-ready.sh"
LEDGER="${ROOT}/docs/specs/migration-ledger.json"

violations=0

report_violation() {
  echo "VIOLATION: $*"
  violations=$((violations + 1))
}

echo "=== check-no-ts-stdio-mcp $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="

[[ -f "${BIN}" ]] || report_violation "missing bin/architecture-reader-mcp"
[[ -f "${RUST_MAIN}" ]] || report_violation "missing crates/architecture-reader-mcp-server/src/main.rs"
[[ -f "${TS_ADAPTER_GATE}" ]] || report_violation "missing scripts/check-ts-adapter-deletion-ready.sh"
[[ -f "${LEDGER}" ]] || report_violation "missing docs/specs/migration-ledger.json"

if [[ -f "${TS_ENTRY}" ]]; then
  report_violation "packages/mcp-server/src/index.ts must be deleted (transport/stdio-ts-adapter ts_deleted)"
fi

if [[ -f "${BIN}" ]]; then
  if ! grep -q 'resolve_rust_bin' "${BIN}"; then
    report_violation "bin must resolve Rust rmcp via resolve_rust_bin"
  fi
  if grep -q 'use_ts_transport' "${BIN}"; then
    report_violation "bin must not retain use_ts_transport after ts_deleted"
  fi
  if grep -qE 'exec (bun|node)' "${BIN}"; then
    report_violation "bin must not exec bun/node after ts_deleted"
  fi
fi

if [[ -f "${RUST_MAIN}" ]]; then
  if ! grep -q 'rmcp::transport::stdio' "${RUST_MAIN}"; then
    report_violation "Rust main must serve rmcp stdio transport"
  fi
fi

if [[ -f "${LEDGER}" ]]; then
  node - "${LEDGER}" <<'NODE'
const [ledgerPath] = process.argv.slice(2);
const ledger = JSON.parse(require("node:fs").readFileSync(ledgerPath, "utf8"));
const tsAdapter = ledger.capabilities.find((cap) => cap.id === "transport/stdio-ts-adapter");
const stdioRust = ledger.capabilities.find((cap) => cap.id === "transport/stdio-rust-rmcp");
if (!tsAdapter || tsAdapter.state !== "ts_deleted") {
  console.error(
    `[check-no-ts-stdio-mcp] transport/stdio-ts-adapter is ${tsAdapter?.state}; expected ts_deleted`
  );
  process.exit(1);
}
if (!stdioRust || !["rust_impl", "authority_rust", "ts_deleted"].includes(stdioRust.state)) {
  console.error(
    `[check-no-ts-stdio-mcp] transport/stdio-rust-rmcp is ${stdioRust?.state}; expected rust authority state`
  );
  process.exit(1);
}
NODE
fi

if [[ "${violations}" -gt 0 ]]; then
  echo ""
  echo "FAIL: ${violations} TS stdio MCP authority violation(s)."
  exit 1
fi

echo "PASS: MCP stdio transport delegates solely to Rust rmcp (no TS adapter)."
