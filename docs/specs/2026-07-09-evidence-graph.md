# Evidence Graph Spec

## Purpose

The architecture evidence graph is the canonical runtime model for repository
architecture answers. It stores architecture-level entities, relationships, and
claims with provenance.

## Graph Object

```json
{
  "schemaVersion": "0.1.0",
  "repository": {
    "root": "/abs/path",
    "gitCommit": "abc123",
    "worktreeDirty": false
  },
  "extractors": [],
  "nodes": [],
  "edges": [],
  "claims": [],
  "evidence": []
}
```

## Node Types

Core node types:

- `repository`
- `workspace`
- `package`
- `service`
- `module`
- `file`
- `symbol`
- `route`
- `endpoint`
- `schema`
- `table`
- `queue`
- `event`
- `workflow`
- `job`
- `config`
- `runtime_binding`
- `test`
- `document`
- `adr`
- `concept`
- `external_dependency`

Nodes must have stable IDs derived from normalized path, symbol identity, or
manifest identity. Do not use array position as a durable ID.

## Edge Types

Structural:

- `contains`
- `imports`
- `exports`
- `depends_on`
- `owns_boundary`
- `documents`
- `tests`

Runtime:

- `calls`
- `routes_to`
- `serves`
- `reads_from`
- `writes_to`
- `publishes`
- `subscribes`
- `configures`
- `deploys`
- `runs`

Derivation:

- `derived_from`
- `inferred_from`
- `conflicts_with`
- `similar_to`

Edges are directed unless the edge type explicitly says otherwise. Every edge
must reference at least one evidence ID.

## Claim Model

Claims represent architecture statements that may combine several facts:

```json
{
  "id": "claim:auth-boundary",
  "text": "Authentication is enforced in the API middleware layer.",
  "confidence": "derived",
  "nodeIds": ["node:module:src/middleware/auth.ts"],
  "edgeIds": ["edge:routes_to:api:auth"],
  "evidenceIds": ["ev_01", "ev_02"],
  "conflicts": []
}
```

Claims are useful for overview answers. They must never replace the underlying
nodes, edges, and evidence.

## Evidence Model

```json
{
  "id": "ev_01",
  "kind": "ast",
  "path": "src/middleware/auth.ts",
  "startLine": 1,
  "endLine": 80,
  "extractor": "synth-typescript",
  "extractorVersion": "0.3.x",
  "contentHash": "sha256:...",
  "confidence": "deterministic"
}
```

Evidence kinds:

- `manifest`
- `ast`
- `import_graph`
- `schema`
- `workflow`
- `config`
- `documentation`
- `adr`
- `test`
- `git`
- `inference`

## Confidence Rules

- `deterministic`: direct parse or manifest evidence.
- `derived`: computed from deterministic graph relationships.
- `inferred`: model or heuristic inference with evidence support.
- `conflicting`: two or more evidence refs disagree.
- `unknown`: evidence exists but extractor cannot classify confidence.

Tools must prefer deterministic and derived evidence over inferred claims.

## Freshness

Graph freshness is keyed by:

- git commit;
- working tree dirty state;
- extractor versions;
- file content hashes;
- graph schema version.

If any of these move in a way the index cannot prove safe, status becomes
`stale` or `dirty`.

## Ranking Signals

Search ranking can use:

- exact ID/path/symbol matches;
- node type match;
- graph centrality within the scoped subgraph;
- edge proximity to query focus;
- lexical score over names, summaries, and docs;
- optional semantic score;
- evidence confidence;
- freshness.

Ranking must expose enough metadata for agent debugging.
