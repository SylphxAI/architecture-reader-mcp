# Benchmarks

## Search quality

<!-- SEARCH:START -->
Run [36781120651](https://github.com/SylphxAI/repomap/actions/runs/36781120651), frozen defaults at `0f8bdad`, 4 vCPU `ubuntu-latest`. NDCG@10, where higher is better and 1.0 means every relevant file is at the top.

**semble's public benchmark**: 63 repositories, 19 languages, 1,251 questions.

| Method | NDCG@10 | architecture | semantic | symbol | index, mean | query p50 |
|---|---:|---:|---:|---:|---:|---:|
| repomap, frozen defaults | 0.845 | 0.807 | 0.848 | 0.925 | 199 ms | 2.90 ms |
| repomap, keywords only (`REPOMAP_EMBED=0`) | 0.802 | 0.750 | 0.803 | 0.917 | 163 ms | 2.22 ms |
| repomap 1.2.2 | 0.685 | 0.614 | 0.668 | 0.883 | 159 ms | 2.17 ms |
| semble, same runner | 0.851 | | | | 1,735 ms | 5.97 ms |

semble's README also lists results from its authors' machine: CodeRankEmbed (a 137M-parameter transformer) 0.839, ColGREP 0.693, BM25 0.673, codebase-memory-mcp 0.630, ripgrep 0.126. These are cited, not re-run.

**Large repositories**: 60 questions of our own over Django, Kubernetes, VS Code and rust-analyzer.

| Method | mean | django | kubernetes | rust-analyzer | vscode | index, mean |
|---|---:|---:|---:|---:|---:|---:|
| repomap, frozen defaults | 0.846 | 0.951 | 0.946 | 0.779 | 0.707 | 6.9 s |
| repomap, keywords only | 0.796 | 0.912 | 0.867 | 0.742 | 0.661 | 5.8 s |
| repomap 1.2.2 | 0.463 | 0.486 | 0.567 | 0.386 | 0.412 | 5.5 s |
| semble, same runner | 0.799 | 0.829 | 0.824 | 0.712 | 0.831 | 66.3 s |

What this shows:
- On the public set, semble wins: 0.851 versus repomap 0.845. The previous published repomap score was 0.851 ([run 36204314484](https://github.com/SylphxAI/repomap/actions/runs/36204314484)); this localization-focused change trades about 0.006 NDCG@10 here. The earlier score is a separate execution, not the 1.2.2 baseline in this table.
- On large repositories, repomap improves from the previous published 0.794 to 0.846, ahead of semble at 0.799. semble still wins on VS Code (0.831 versus 0.707); repomap wins on Django, Kubernetes and rust-analyzer.
- Embeddings add about 0.043 on the public set and 0.050 on large repositories.
- TypeScript remains the weakest language. No weights were selected on either of these question sets in this outcome.

<details>
<summary>By language (frozen defaults, public set)</summary>

| Language | NDCG@10 |
|---|---:|
| bash | 0.829 |
| c | 0.795 |
| cpp | 0.893 |
| csharp | 0.899 |
| elixir | 0.908 |
| go | 0.885 |
| haskell | 0.813 |
| java | 0.812 |
| javascript | 0.901 |
| kotlin | 0.782 |
| lua | 0.882 |
| php | 0.844 |
| python | 0.876 |
| ruby | 0.834 |
| rust | 0.833 |
| scala | 0.880 |
| swift | 0.823 |
| typescript | 0.721 |
| zig | 0.851 |

</details>
<!-- SEARCH:END -->

### Method

- **Question sets.**
  - [semble's benchmark](https://github.com/MinishLab/semble/tree/24497845460960db1839c8485319df189a889225/benchmarks), at commit `2449784`: 1,251 questions over 63 open-source repositories in 19 languages, each pinned to a commit. The questions are sorted into architecture ("how are routes registered"), semantic ("session management") and symbol (`Blueprint`). semble's authors wrote the questions and relevant files with Claude Sonnet 4.6 and checked them with an LLM judge. We use the set as published.
  - Our [large-repository set](https://github.com/SylphxAI/repomap/tree/main/bench/search): 60 plain-language questions over Django, Kubernetes, VS Code and rust-analyzer, at the commits of the speed benchmark below. We wrote the questions and answers before running any search, and checked every answer path against the pinned tree. Because we wrote this set, read it together with the public one.
- **Scoring**, the same as semble's `run_benchmark.py`:
  - Take the top 10 results of the MCP `search` tool.
  - A target file counts as found at the rank of the first result in that file whose lines overlap the target's lines, if the target gives any.
  - NDCG@10 is averaged per repository, then per language, then over the languages.
- **Runs.** On one GitHub-hosted `ubuntu-latest` runner in the [`bench` workflow](https://github.com/SylphxAI/repomap/actions/workflows/bench.yml):
  - this build;
  - this build with `REPOMAP_EMBED=0` (keywords only);
  - repomap 1.2.2, an older release baseline;
  - semble itself, through its own harness, on the same repositories.
- **Speed columns.**
  - repomap's index time is `repomap index --no-cache` as a new process, which includes loading the model.
  - repomap's query time is a round trip over MCP stdio.
  - semble's numbers come from its harness: indexing in-process, and a Python function call per query.
  - So the speed columns show the size of the costs. They are not a race.
- **Tuning.** The older 1.3 ranking had been tuned on 20 of the 63 repositories. This localization-focused outcome selected changes only on the disjoint 300-instance SWE-bench tuning split below, froze them, then evaluated these question sets without tuning on their results. The public set is therefore a regression check, not a wholly unseen benchmark for the historical ranking.

## File localization

### Tuning split (300 instances, not Verified)

Defaults were selected **only** on `manifest-tune.json`: 300 instances from the pinned full SWE-bench test set (`e48e2bd`), excluding every Verified instance, evenly spaced by instance id. This is not SWE-bench Lite. Gold files score the candidates; the search query is only the issue text, truncated to 20,000 characters for every method.

Two tuning runs tested 26 individual changes and 10 combinations. We selected the highest Acc@5 + Acc@10 among candidates within 2× the old binary's median query time, then froze the defaults before measuring Verified: `wh=1,wc=1,wf=2,qcap=48` (title and code embedding lists, file BM25 weight 2, at most 48 weighted lexical terms). Other defaults are unchanged from the tuning implementation.

| Method | Acc@1 | Acc@5 | Acc@10 | index, median | query, median |
|---|---:|---:|---:|---:|---:|
| repomap before | 19.7 | 42.0 | 53.0 | 1.755 s | 48.5 ms |
| **repomap, frozen defaults** | **41.3** | **64.3** | **73.0** | **1.739 s** | **45.0 ms** |
| semble | 30.7 | 52.7 | 62.0 | 13.338 s | 342.9 ms |
| BM25 over files | 16.7 | 34.7 | 43.7 | 2.384 s | 68.6 ms |

Before and selected repomap are from run [36779097206](https://github.com/SylphxAI/repomap/actions/runs/36779097206), same runner shards. semble and BM25 are from the earlier tuning run [36525197903](https://github.com/SylphxAI/repomap/actions/runs/36525197903), on the identical split and standard runner type, not the same execution. Every method scored 300/300 with zero errors. Selection-set gains are not held-out evidence; use Verified below for that. The selected query median is 0.93× before, and index median 0.99×. Full ranks, timings, experiment summaries and run provenance are in [`results-tune.json`](https://github.com/SylphxAI/repomap/blob/main/bench/localization/results-tune.json).

### SWE-bench Verified (500 instances)

Given a GitHub issue, does `search` rank the files the fix touches near the top? This runs on [SWE-bench Verified](https://huggingface.co/datasets/princeton-nlp/SWE-bench_Verified) (500 instances, 12 Python repositories, dataset revision `c104f84`). Each repository is checked out at the instance's base commit, and each method gets the issue text as its only query, in one call. Files are ranked by their best-ranked result.

- **Acc@k**: every file the gold patch modifies is among the top k files (the definition LocAgent and Agentless use).
- **Hit@k**: at least one of them is.
- **Chunk@k**: one of the top k result chunks overlaps a line the gold patch changes (repomap and semble only, since BM25 over files returns no line ranges).

<!-- LOC:START -->
Run [36781120696](https://github.com/SylphxAI/repomap/actions/runs/36781120696), frozen defaults at `0f8bdad`, 4 vCPU `ubuntu-latest`, 10 shards. Every method scored 500/500 with zero errors. 500 instances scored.

| Method | Acc@1 | Acc@5 | Acc@10 | Hit@1 | Hit@5 | Hit@10 | Chunk@5 | Chunk@10 | index, median | query, median |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| repomap `search`, before (`main`) | 23.6 | 51.8 | 62.2 | 27.6 | 58.6 | 69.2 | 29.8 | 35.4 | 1.7 s | 60 ms |
| repomap `search` | 48.8 | 74.8 | 83.0 | 56.2 | 83.6 | 89.6 | 42.8 | 51.6 | 1.7 s | 54 ms |
| repomap, keywords only (`REPOMAP_EMBED=0`) | 43.8 | 71.0 | 77.6 | 50.6 | 79.2 | 84.0 | 43.4 | 47.6 | 1.5 s | 45 ms |
| BM25 over files | 20.8 | 45.4 | 55.8 | 23.2 | 51.0 | 62.8 |  |  | 2.3 s | 72 ms |
| semble | 34.0 | 61.6 | 71.0 | 39.4 | 69.6 | 79.2 | 20.2 | 25.6 | 12.8 s | 354 ms |

Acc@10 by repository (every gold file in the top 10 files):

| Repository | instances | repomap `search`, before (`main`) | repomap `search` | repomap, keywords only (`REPOMAP_EMBED=0`) | BM25 over files | semble | repomap index, median | repomap query, median |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| astropy/astropy | 22 | 63.6 | 86.4 | 81.8 | 63.6 | 63.6 | 2.1 s | 55 ms |
| django/django | 231 | 63.2 | 85.7 | 79.2 | 55.0 | 73.6 | 1.7 s | 65 ms |
| matplotlib/matplotlib | 34 | 64.7 | 82.4 | 76.5 | 50.0 | 58.8 | 1.8 s | 47 ms |
| mwaskom/seaborn | 2 | 100.0 | 100.0 | 100.0 | 100.0 | 100.0 | 0.3 s | 10 ms |
| pallets/flask | 1 | 100.0 | 100.0 | 100.0 | 0.0 | 100.0 | 0.1 s | 4 ms |
| psf/requests | 8 | 100.0 | 100.0 | 100.0 | 62.5 | 100.0 | 0.2 s | 4 ms |
| pydata/xarray | 22 | 72.7 | 86.4 | 81.8 | 54.5 | 77.3 | 0.5 s | 22 ms |
| pylint-dev/pylint | 10 | 40.0 | 40.0 | 30.0 | 30.0 | 30.0 | 0.5 s | 23 ms |
| pytest-dev/pytest | 19 | 47.4 | 68.4 | 73.7 | 47.4 | 68.4 | 0.3 s | 19 ms |
| scikit-learn/scikit-learn | 32 | 81.2 | 93.8 | 93.8 | 81.2 | 90.6 | 1.1 s | 41 ms |
| sphinx-doc/sphinx | 44 | 31.8 | 75.0 | 65.9 | 36.4 | 50.0 | 0.8 s | 27 ms |
| sympy/sympy | 75 | 65.3 | 80.0 | 74.7 | 64.0 | 74.7 | 4.7 s | 59 ms |
<!-- LOC:END -->

What this shows:
- **The file-localization goal is met.** Acc@5 improves from 51.8 to 74.8 and Acc@10 from 62.2 to 83.0: +23.0 and +20.8 percentage points over before, and +13.2 and +12.0 over semble. Acc@1 rises from 23.6 to 48.8, versus semble's 34.0.
- **Speed stays within 2×.** In this same execution, the old binary's median index/query costs are 1.686 s / 59.8 ms, versus 1.682 s / 54.4 ms for the new binary (1.00× / 0.91×). semble's costs are 12.813 s / 354.4 ms. Index time includes model loading; repomap queries are warmed MCP round trips, while semble queries are in-process Python calls.
- The gains are not just embeddings: keywords-only repomap reaches 71.0 / 77.6 at Acc@5/10. Embeddings add another 3.8 / 5.4 points. Chunk@10 improves to 51.6, versus 35.4 before and 25.6 for semble.
- repomap meets or exceeds semble's Acc@10 on every repository here, but this does **not** make it better on every query or benchmark. semble still wins on the public NDCG@10 set and on VS Code's large-repository questions above. Hybrid search also trails its own keywords-only result on pytest (68.4 versus 73.7 Acc@10).
- Verified was measured once after freezing defaults at `0f8bdad`; no weights were changed after reading its results. The before binary is pinned to `ae40b64dbc1405564e21676871a53812e2cf4ad5`, not a moving main branch. Full ranks and timings are in [`results.json`](https://github.com/SylphxAI/repomap/blob/main/bench/localization/results.json).
- This is one query per instance with the raw issue text. It is not an agent, so it is not comparable with the published agent numbers below.

Published file-level numbers on SWE-bench **Lite** (300 instances, a different subset with a different setup, the agents use a language model in a loop). Self-reported, different setup; we did not re-run them. From Table 4 of [LocAgent](https://arxiv.org/abs/2503.09089) (ACL 2025):

| System | Acc@1 | Acc@5 |
|---|---:|---:|
| BM25 | 38.7 | 61.7 |
| CodeRankEmbed ([CoRNStack](https://arxiv.org/abs/2412.01007)) | 52.6 | 84.7 |
| Agentless, Claude-3.5 ([paper](https://arxiv.org/abs/2407.01489)) | 72.6 | 79.6 |
| LocAgent, Claude-3.5 | 77.7 | 94.2 |

Even plain BM25 scores higher there than on our run, which shows that the two subsets and setups differ: read these numbers next to each other only as context.

Method: [`bench/localization`](https://github.com/SylphxAI/repomap/tree/main/bench/localization), run by the [`bench-localization` workflow](https://github.com/SylphxAI/repomap/actions/workflows/bench-localization.yml) on `ubuntu-latest` runners in 10 shards. BM25 is `rank_bm25` over the tracked text files, with identifiers split at camelCase and underscores. semble is at commit `2449784`. repomap ran with frozen defaults selected only on the disjoint tuning split above, never on Verified.

## Indexing speed

Measured by [`scripts/bench.py`](https://github.com/SylphxAI/repomap/blob/main/scripts/bench.py) in the [`bench` workflow](https://github.com/SylphxAI/repomap/actions/workflows/bench.yml) on a standard GitHub-hosted `ubuntu-latest` runner. Anyone can re-run it.

<!-- BENCH:START -->
Run [36781120651](https://github.com/SylphxAI/repomap/actions/runs/36781120651) (frozen search defaults, `0f8bdad`, 4 vCPU `ubuntu-latest`), with embeddings and with `REPOMAP_EMBED=0`:

| Repository | Code files | Symbols | Resolved calls | Cold index | Cold, keywords only | Warm index | Peak RSS |
|---|---:|---:|---:|---:|---:|---:|---:|
| kubernetes (Go) | 11,710 | 101,176 | 200,256 | 13.1 s | 11.0 s | 1.96 s | 574 MB |
| vscode (TypeScript) | 6,126 | 95,786 | 182,506 | 9.3 s | 8.0 s | 1.45 s | 408 MB |
| django (Python) | 2,271 | 41,019 | 59,643 | 2.6 s | 2.2 s | 0.47 s | 170 MB |
| rust-analyzer (Rust) | 1,512 | 29,024 | 59,438 | 2.1 s | 1.8 s | 0.37 s | 152 MB |

Query latency over MCP stdio, p50 / p95:

| Repository | search | context | impact | trace |
|---|---:|---:|---:|---:|
| kubernetes | 73.7 / 85.7 ms | 0.8 / 4.4 ms | 5.7 / 10.6 ms | 0.6 / 8.1 ms |
| vscode | 16.8 / 54.8 ms | 2.0 / 2.8 ms | 7.2 / 10.5 ms | 0.5 / 0.6 ms |
| django | 23.9 / 28.8 ms | 0.4 / 2.1 ms | 0.9 / 3.6 ms | 0.3 / 0.4 ms |
| rust-analyzer | 7.7 / 22.1 ms | 0.6 / 1.2 ms | 2.0 / 3.4 ms | 0.4 / 0.5 ms |

Against the previous published run ([36202944872](https://github.com/SylphxAI/repomap/actions/runs/36202944872)), search p50 is 0.98–1.05× and cold index 1.01–1.03× across these four repositories, comfortably within 2×. These are separate executions on the same runner type; the localization table compares old and new binaries within the same execution.
<!-- BENCH:END -->

## Method

- **Corpora:** shallow clones at fixed tags: microsoft/vscode `1.104.0` (TypeScript), rust-lang/rust-analyzer `2026-09-21` (Rust), django/django `5.2.6` (Python) and kubernetes/kubernetes `v1.34.1` (Go).
- **Cold index:** `repomap index <repo> --no-cache`, the median of 3 runs. This covers the walk, tree-sitter parse, embeddings, import and call resolution, PageRank, Louvain and BM25.
- **Warm index:** the same command with the per-file cache populated. This is what a restart of the MCP server costs.
- **Peak RSS:** the highest resident memory of the index runs.
- **Query latency:** one long-lived `repomap mcp` process, with the round trip measured over stdio JSON-RPC. `search` runs 5 phrases plus the 10 most used symbol names. `context`, `impact` and `trace` (callers) each run on those 10 symbols. The table shows p50; p95 is in the JSON artifact.

## Versus GitNexus

We have not run GitNexus in this benchmark. Its PolyForm Noncommercial licence does not allow use for a company's commercial purposes, and a vendor benchmark could count as one. Its README documents its own performance. You are welcome to run both tools on the same corpora on your own machine.
