# File localization on SWE-bench Verified

For each of the 500 instances of [SWE-bench Verified](https://huggingface.co/datasets/princeton-nlp/SWE-bench_Verified)
(dataset revision `c104f84`, MIT-licensed), the repository is checked out at
`base_commit`, and each method gets the issue text as its only query in one call.
We score whether the files the gold patch modifies are among the top ranked files.

- `manifest.json`: the pinned dataset revision, the gold files and changed line ranges per instance, and the 50-instance smoke set (every 10th by id). Made by `localize.py prepare`.
- `localize.py`: `prepare`, `run` (one shard) and `summarize` (merge shards, write the tables into `docs/benchmarks.md`). Metric definitions are in its header.
- `results.json`: per-instance ranks and timings of the last full run.

Methods: repomap `search` (with and without embeddings), BM25 over the tracked text files of the checkout (`rank_bm25`, code-aware tokens), and semble at its pinned commit.

Run it: Actions, `bench-localization`, "Run workflow", mode `full`. It builds repomap, runs 10 shards on `ubuntu-latest` and merges them into the `localization` artifact. Nothing here tunes on the instances: repomap runs as released.
