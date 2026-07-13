#!/usr/bin/env bash
# architecture-reader-mcp pure residual differential — TS pure-contract oracle vs Rust rmcp SSOT.
# Slices: pure-residual (default) | all
# Fail-closed: requires bun OR node>=22 strip-types. No SKIP-as-pass.
# Explicit non-claims: architecture_* graph effect parity, HTTP, parity_proven,
# authority_rust, ts_deleted. See rej-010 / BW2 residual.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRATCH="${SCRATCH_DIR:-/tmp/architecture-reader-mcp-differential}"
mkdir -p "$SCRATCH"
LOG="$SCRATCH/differential.log"
ORACLE_JSON="$SCRATCH/oracle.json"
SLICE_FILTER="pure-residual"
: >"$LOG"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --slice)
      SLICE_FILTER="${2:-}"
      shift 2
      ;;
    *)
      echo "::error::unknown argument: $1" | tee -a "$LOG"
      exit 1
      ;;
  esac
done

case "$SLICE_FILTER" in
  all|pure-residual) ;;
  *)
    echo "::error::invalid --slice value: $SLICE_FILTER (supported: pure-residual|all)" | tee -a "$LOG"
    exit 1
    ;;
esac

cd "$REPO_ROOT"

run_oracle() {
  local script="$REPO_ROOT/scripts/differential/architecture-reader-mcp-oracle.ts"
  if command -v bun >/dev/null 2>&1; then
    bun run "$script"
  elif command -v node >/dev/null 2>&1; then
    node --experimental-strip-types "$script"
  else
    echo "::error::bun or node>=22 required for architecture-reader-mcp differential — no SKIP-as-pass" | tee -a "$LOG"
    exit 1
  fi
}

echo "=== architecture-reader-mcp pure differential $(date -u +%Y-%m-%dT%H:%M:%SZ) slice=$SLICE_FILTER ===" | tee -a "$LOG"

echo "--- build Rust core + rmcp server ---" | tee -a "$LOG"
cargo build -p architecture-reader-core -p architecture-reader-mcp-server 2>&1 | tee -a "$LOG"

echo "--- TS pure-contract oracle ---" | tee -a "$LOG"
run_oracle >"$ORACLE_JSON" 2>>"$LOG"

echo "--- Rust bounded pure residual differential ---" | tee -a "$LOG"
ARCHITECTURE_READER_MCP_ORACLE_JSON="$ORACLE_JSON" \
  cargo test -p architecture-reader-mcp-server --test architecture_reader_mcp_differential \
  pure_residual_differential_matches_ts_oracle -- --nocapture 2>&1 | tee -a "$LOG"

CANDIDATE_SHA="${CANDIDATE_SHA:-$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)}"
if command -v sha256sum >/dev/null 2>&1; then
  BEHAVIOR_SPEC_HASH="$(sha256sum "$REPO_ROOT/scripts/differential/fixtures/architecture-reader-mcp-corpus.json" | awk '{print $1}')"
else
  BEHAVIOR_SPEC_HASH="$(shasum -a 256 "$REPO_ROOT/scripts/differential/fixtures/architecture-reader-mcp-corpus.json" | awk '{print $1}')"
fi
FIXTURE_CORPUS_HASH="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["fixtureCorpusHash"])' "$ORACLE_JSON")"
CASE_COUNT="$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["cases"]))' "$ORACLE_JSON")"

export CANDIDATE_SHA BEHAVIOR_SPEC_HASH FIXTURE_CORPUS_HASH CASE_COUNT SLICE_FILTER SCRATCH

python3 - <<'PY'
import json, os, datetime, pathlib
scratch = pathlib.Path(os.environ["SCRATCH"])
payload = {
  "schemaVersion": 2,
  "slice": "architecture-reader-mcp.pure-residual|" + os.environ.get("SLICE_FILTER", "pure-residual"),
  "status": "differential_green",
  "verifiedAt": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
  "lastComparedMainSha": os.environ["CANDIDATE_SHA"],
  "mergeGroupSha": os.environ["CANDIDATE_SHA"],
  "rustCandidateSha": os.environ["CANDIDATE_SHA"],
  "behaviorSpecHash": os.environ["BEHAVIOR_SPEC_HASH"],
  "fixtureCorpusHash": os.environ["FIXTURE_CORPUS_HASH"],
  "caseCount": int(os.environ["CASE_COUNT"]),
  "harness": "scripts/run-architecture-reader-differential.sh",
  "differentialTest": "crates/architecture-reader-mcp-server/tests/architecture_reader_mcp_differential.rs#pure_residual_differential_matches_ts_oracle",
  "boundedSlices": {
    "tool-route-contract": "pure_residual_differential_matches_ts_oracle",
    "server-contract": "pure_residual_differential_matches_ts_oracle",
    "allow-list": "pure_residual_differential_matches_ts_oracle"
  },
  "oracle": "scripts/differential/architecture-reader-mcp-oracle.ts",
  "allowList": [
    "architecture_index",
    "architecture_status",
    "architecture_overview",
    "architecture_search",
    "architecture_trace",
    "architecture_impact",
    "architecture_evidence"
  ],
  "promotionPolicy": "NO_PROMOTIONS — pure residual differential_green only; NOT graph effect parity; NOT authority_rust; NOT HTTP; NOT ts_deleted; rej-010 hold remains"
}
path = scratch / "verification.json"
path.write_text(json.dumps(payload, indent=2) + "\n")
print(f"verification artifact: {path}")
PY

mkdir -p "$REPO_ROOT/docs/specs/verification"
cp "$SCRATCH/verification.json" "$REPO_ROOT/docs/specs/verification/bw2-tip-pure-residual-differential.json"

echo "architecture-reader-mcp-differential: OK (slice=$SLICE_FILTER cases=$CASE_COUNT corpus=$FIXTURE_CORPUS_HASH)" | tee -a "$LOG"
echo "verification artifact: $SCRATCH/verification.json" | tee -a "$LOG"
