# Indexing Pipeline Spec

## Pipeline Goals

The indexer creates an architecture evidence graph from local repository files.
It must be deterministic by default, incremental where safe, and explicit about
coverage gaps.

## Phases

### Phase 0: Preflight

- resolve repository root;
- identify git commit and dirty state;
- read ignore rules;
- load previous index metadata if present;
- determine full rebuild vs incremental refresh.

### Phase 1: Inventory

- enumerate files;
- classify files by category: source, manifest, docs, schema, workflow, config,
  infra, test, generated, vendor;
- record file hashes, sizes, mtimes, and language hints;
- enforce max file size.

### Phase 2: Deterministic Extraction

Run extractor adapters:

- manifest/workspace extractor;
- Synth AST extractor;
- import/export extractor;
- docs/ADR extractor;
- schema/config/workflow extractor;
- test relationship extractor.

Extractor failures become graph gaps, not silent omissions.

### Phase 3: Normalize

- convert extractor facts into canonical nodes, edges, claims, and evidence;
- de-duplicate nodes;
- resolve local imports and package references;
- attach evidence to every edge;
- detect conflicts.

### Phase 4: Index

Build derived indexes:

- node ID/path/symbol lookup;
- edge adjacency;
- relation-specific adjacency;
- lexical index over node names, paths, summaries, docs, and claims;
- optional semantic vectors;
- impact reverse index.

### Phase 5: Validate

Validation gates:

- all edges reference existing nodes;
- all claims reference existing evidence;
- all evidence paths exist or are marked deleted/stale;
- extractor versions are recorded;
- graph schema version matches the reader;
- index commit is recorded.

### Phase 6: Persist

Persist graph snapshot, derived indexes, file hashes, extractor versions, and
validation report.

## Incremental Refresh

Incremental refresh is allowed when:

- graph schema version is unchanged;
- extractor major versions are unchanged;
- changed files are within supported categories;
- deleted files can be removed cleanly;
- reverse indexes can be updated without full recomputation.

Otherwise the indexer must perform a full rebuild or report why it cannot.

## Large Repository Behavior

Large repositories require:

- batch processing;
- memory-bounded graph assembly;
- file size limits;
- generated/vendor exclusion defaults;
- progress reporting;
- partial coverage report when extraction is incomplete.

## Extractor Contract

Each extractor returns:

```json
{
  "name": "synth-typescript",
  "version": "0.3.x",
  "facts": [],
  "evidence": [],
  "gaps": []
}
```

Extractors must avoid global mutable assumptions. They receive a file or batch,
repository metadata, and config; they return facts plus evidence.

## Storage Decision Deferred

The graph model must be tested before locking the storage backend. Candidate
storage:

- SQLite for simple portable snapshots;
- Tantivy plus binary graph files for high-performance search;
- RocksDB or sled for large incremental indexes;
- JSONL for debug exports.

The first implementation should use the simplest backend that passes golden
fixture tests and can be replaced behind the Rust engine boundary.
