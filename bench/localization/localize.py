#!/usr/bin/env python3
"""File localization on SWE-bench Verified: does search put the files a fix touches near the top?

usage: localize.py prepare <manifest.json>
           Download the pinned dataset and write the manifest (gold files and hunks).
       localize.py run <repomap-binary> <manifest.json> <work-dir> <out.json>
                       [--shard I/N] [--smoke] [--only ID ...] [--skip keywords,bm25,semble]
                       [--baseline <binary>] [--variants <json of name: REPOMAP_TUNE>]
           Score every method on the instances of one shard.
       localize.py summarize <manifest.json> <out.json> <shard.json ...> [--docs docs/benchmarks.md]
           Merge shards into the results file and print (or write into the docs) the tables.

For each instance the repository is checked out at `base_commit`, and each method
gets the issue text (`problem_statement`) as its only query, one search call.
Files are ranked by first appearance in the results. Scoring:

  Acc@k   every file the gold patch modifies is in the top k files (LocAgent's definition)
  Hit@k   at least one of them is
  Chunk@k at least one of the top k result chunks overlaps a line the gold patch changes
          (repomap and semble only: they return line ranges, and their chunks are
          functions, classes or blocks; BM25 over files does not)
"""
import json
import math
import os
import re
import statistics
import subprocess
import sys
import tempfile
import time
from collections import defaultdict

DATASET = "princeton-nlp/SWE-bench_Verified"
REVISION = "c104f840cc67f8b6eec6f759ebc8b2693d585d4a"
# The tuning split: instances of the full SWE-bench test set that are not in Verified.
FULL_DATASET = "princeton-nlp/SWE-bench"
FULL_REVISION = "e48e2bd1e9fecd5bbd641e9414ac59da9f2e69f6"
TUNE_SIZE = 300
KS = (1, 5, 10)
MAX_QUERY_CHARS = 20000
METHODS = ["main", "repomap", "repomap-keywords", "bm25", "semble"]
LABELS = {
    "main": "repomap `search`, before (`main`)",
    "repomap": "repomap `search`",
    "repomap-keywords": "repomap, keywords only (`REPOMAP_EMBED=0`)",
    "bm25": "BM25 over files",
    "semble": "semble",
}


# ---------- dataset and manifest ----------

def hunks_of(patch):
    """Changed old-file lines per file, one range per run of changed lines."""
    out = defaultdict(list)
    cur, old, run = None, 0, None
    for line in patch.splitlines():
        if line.startswith("--- "):
            p = line[4:].split("\t")[0]
            cur = p[2:] if p.startswith("a/") else None
            run = None
        elif line.startswith("+++ "):
            continue
        elif line.startswith("@@") and cur:
            old = int(re.match(r"@@ -(\d+)", line).group(1))
            run = None
        elif cur and line[:1] in ("-", "+", " ") and not line.startswith("\\"):
            if line[0] == " ":
                old += 1
                run = None
            elif line[0] == "-":
                if run is None:
                    run = [old, old]
                    out[cur].append(run)
                run[1] = old
                old += 1
            else:  # insertion: anchored on the line it lands before
                if run is None:
                    run = [max(old - 1, 1), old]
                    out[cur].append(run)
    return {k: v for k, v in out.items()}


def load_dataset(dataset=DATASET, revision=REVISION):
    import pyarrow.parquet as pq
    import urllib.request
    path = os.path.join(tempfile.gettempdir(), f"{dataset.replace('/', '-')}-{revision[:12]}.parquet")
    if not os.path.exists(path):
        urllib.request.urlretrieve(f"https://huggingface.co/datasets/{dataset}/resolve/{revision}/data/test-00000-of-00001.parquet", path)
    return pq.read_table(path).to_pylist()


def prepare(out, split="verified"):
    if split == "tune":
        verified = {r["instance_id"] for r in load_dataset()}
        rest = sorted((r for r in load_dataset(FULL_DATASET, FULL_REVISION) if r["instance_id"] not in verified), key=lambda r: r["instance_id"])
        rows = [r for r in rest if hunks_of(r["patch"])]
        step = len(rows) / TUNE_SIZE
        rows = [rows[int(i * step)] for i in range(TUNE_SIZE)]  # evenly spaced by id: every repository in proportion
        dataset, revision = FULL_DATASET, FULL_REVISION
    else:
        rows = sorted(load_dataset(), key=lambda r: r["instance_id"])
        dataset, revision = DATASET, REVISION
    inst = []
    for r in rows:
        h = hunks_of(r["patch"])
        inst.append({"id": r["instance_id"], "repo": r["repo"], "base_commit": r["base_commit"], "gold": h})
    ids = [i["id"] for i in inst]
    manifest = {
        "dataset": dataset,
        "revision": revision,
        "instances": len(inst),
        "smoke": ids[::10],  # every 10th by id: 50 instances across 10 of the 12 repositories
        "items": inst,
    }
    json.dump(manifest, open(out, "w"), indent=1)
    empty = [i["id"] for i in inst if not i["gold"]]
    print(f"{len(inst)} instances, {len(manifest['smoke'])} in the smoke set, {len(empty)} without an existing gold file: {empty}")


# ---------- repositories ----------

def git(repo_dir, *a, check=True):
    return subprocess.run(["git", "-C", repo_dir, *a], check=check, capture_output=True, text=True)


def checkout(work, repo, sha):
    d = os.path.join(work, repo.replace("/", "__"))
    if not os.path.isdir(os.path.join(d, ".git")):
        subprocess.run(["git", "clone", "-q", "--filter=blob:none", "--no-checkout", f"https://github.com/{repo}.git", d], check=True, capture_output=True)
    if git(d, "cat-file", "-e", sha + "^{commit}", check=False).returncode != 0:
        git(d, "fetch", "-q", "origin", sha)
    git(d, "checkout", "-q", "-f", "--detach", sha)
    git(d, "clean", "-fdxq")
    return d


# ---------- methods ----------

def path_matches(a, b):
    return a == b or a.endswith("/" + b) or b.endswith("/" + a)


class Mcp:
    def __init__(self, binary, root, env):
        t = time.perf_counter()
        self.p = subprocess.Popen([binary, "mcp", "--root", root], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, env=env)
        self.id = 0
        self.call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "bench", "version": "1"}})
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        self.search("main")  # warm-up, so the timed query is not the first one
        self.ready_ms = (time.perf_counter() - t) * 1000

    def send(self, m):
        self.p.stdin.write(json.dumps(m) + "\n")
        self.p.stdin.flush()

    def call(self, method, params):
        self.id += 1
        self.send({"jsonrpc": "2.0", "id": self.id, "method": method, "params": params})
        while True:
            line = self.p.stdout.readline()
            if not line:
                raise RuntimeError("repomap mcp exited")
            m = json.loads(line)
            if m.get("id") == self.id:
                return m

    def search(self, query):
        t = time.perf_counter()
        r = self.call("tools/call", {"name": "search", "arguments": {"query": query, "limit": 100, "include_content": False, "format": "json"}})
        ms = (time.perf_counter() - t) * 1000
        res = r["result"]
        if res.get("isError"):
            raise RuntimeError(res["content"][0]["text"][:200])
        return ms, [(h["file"], h["start"], h["end"]) for h in json.loads(res["content"][0]["text"])["hits"]]

    def close(self):
        try:
            self.p.stdin.close()
            self.p.wait(timeout=30)
        except Exception:
            self.p.kill()


def repomap_index(binary, root, embed):
    """Index once into a private cache; returns the environment for later queries and the cold index time."""
    cache = tempfile.mkdtemp(prefix="repomap-loc-")
    env = dict(os.environ, REPOMAP_CACHE_DIR=cache)
    if not embed:
        env["REPOMAP_EMBED"] = "0"
    t = time.perf_counter()
    subprocess.run([binary, "index", root, "--no-cache", "--json"], check=True, capture_output=True, text=True, env=env)
    return env, (time.perf_counter() - t) * 1000


def repomap_query(binary, root, env, query, tune=None):
    if tune:
        env = dict(env, REPOMAP_TUNE=tune)
    mcp = Mcp(binary, root, env)
    try:
        query_ms, chunks = mcp.search(query)
    finally:
        mcp.close()
    return {"ready_ms": mcp.ready_ms, "query_ms": query_ms}, chunks


def run_repomap(binary, root, query, embed, session=None, tune=None):
    env, index_ms = session or repomap_index(binary, root, embed)
    timing, chunks = repomap_query(binary, root, env, query, tune)
    return {"index_ms": index_ms, **timing}, chunks


SKIP_EXT = {".po", ".mo", ".png", ".jpg", ".jpeg", ".gif", ".ico", ".svg", ".pdf", ".zip", ".gz", ".whl", ".pyc", ".so", ".woff", ".woff2", ".ttf", ".lock", ".min.js", ".npy", ".pkl", ".fits", ".dat"}
_word = re.compile(r"[A-Za-z_][A-Za-z0-9_]*|\d+")
_camel = re.compile(r"[A-Z]+(?![a-z])|[A-Z]?[a-z]+|\d+")


def tokens(text):
    out = []
    for w in _word.findall(text):
        lw = w.lower()
        parts = [p.lower() for p in _camel.findall(w.replace("_", " "))]
        out.append(lw)
        if len(parts) > 1:
            out.extend(parts)
    return out


def run_bm25(root, query):
    from rank_bm25 import BM25Okapi
    t = time.perf_counter()
    names = git(root, "ls-files").stdout.splitlines()
    files, docs = [], []
    for n in names:
        p = os.path.join(root, n)
        if os.path.splitext(n)[1].lower() in SKIP_EXT or not os.path.isfile(p) or os.path.getsize(p) > 1_000_000:
            continue
        try:
            raw = open(p, "rb").read()
        except OSError:
            continue
        if b"\0" in raw[:8192]:
            continue
        files.append(n)
        docs.append(tokens(raw.decode("utf-8", "replace")))
    bm = BM25Okapi(docs)
    index_ms = (time.perf_counter() - t) * 1000
    t = time.perf_counter()
    scores = bm.get_scores(tokens(query))
    order = sorted(range(len(files)), key=lambda i: -scores[i])[:100]
    query_ms = (time.perf_counter() - t) * 1000
    return {"index_ms": index_ms, "query_ms": query_ms}, [(files[i], None, None) for i in order]


def run_semble(root, query):
    from semble import SembleIndex
    t = time.perf_counter()
    idx = SembleIndex.from_path(root)
    index_ms = (time.perf_counter() - t) * 1000
    t = time.perf_counter()
    res = idx.search(query, top_k=100)
    query_ms = (time.perf_counter() - t) * 1000
    chunks = []
    for r in res:
        fp = str(r.chunk.file_path)
        if fp.startswith(root):
            fp = os.path.relpath(fp, root)
        chunks.append((fp, r.chunk.start_line, r.chunk.end_line))
    return {"index_ms": index_ms, "query_ms": query_ms}, chunks


# ---------- scoring ----------

def rank_files(chunks):
    seen = []
    for f, _, _ in chunks:
        if f not in seen:
            seen.append(f)
    return seen


def score(gold, chunks):
    files = rank_files(chunks)
    def pos(g):
        return next((i + 1 for i, f in enumerate(files) if path_matches(f, g)), None)
    ranks = {g: pos(g) for g in gold}
    out = {"files": files[:10], "gold_ranks": ranks}
    def covers(c, g):
        f, s, e = c
        return s is not None and any(path_matches(f, g) and not (e < a or s > b) for a, b in gold[g])
    out["chunk_rank"] = None
    for i, c in enumerate(chunks, 1):
        if any(covers(c, g) for g in gold):
            out["chunk_rank"] = i
            break
    if not any(s is not None for _, s, _ in chunks[:1]):
        out["chunk_rank"] = "n/a"
    return out


def run(binary, manifest_path, work, out, shard, smoke, only, skip, baseline, variants):
    m = json.load(open(manifest_path))
    ds = {r["instance_id"]: r["problem_statement"] for r in load_dataset(m["dataset"], m["revision"])}
    items = [i for i in m["items"] if i["gold"]]
    if smoke:
        items = [i for i in items if i["id"] in set(m["smoke"])]
    if only:
        items = [i for i in items if i["id"] in only]
    idx, n = shard
    size = math.ceil(len(items) / n)
    items = items[idx * size:(idx + 1) * size]
    os.makedirs(work, exist_ok=True)
    results = []
    for k, it in enumerate(items, 1):
        query = ds[it["id"]][:MAX_QUERY_CHARS]
        row = {"id": it["id"], "repo": it["repo"], "gold": sorted(it["gold"]), "methods": {}}
        try:
            root = checkout(work, it["repo"], it["base_commit"])
        except Exception as e:
            row["error"] = f"checkout: {e}"
            results.append(row)
            continue
        session = {}

        def sess(key, bin_, embed):
            if key not in session:
                session[key] = repomap_index(bin_, root, embed)
            return session[key]

        runs = {"repomap": lambda: run_repomap(binary, root, query, True, sess("new", binary, True))}
        if baseline:
            runs["main"] = lambda: run_repomap(baseline, root, query, True)
        for vname, tune in variants.items():
            runs[f"repomap:{vname}"] = lambda tune=tune: run_repomap(binary, root, query, True, sess("new", binary, True), tune)
        if "keywords" not in skip:
            runs["repomap-keywords"] = lambda: run_repomap(binary, root, query, False)
        if "bm25" not in skip:
            runs["bm25"] = lambda: run_bm25(root, query)
        if "semble" not in skip:
            runs["semble"] = lambda: run_semble(root, query)
        for name, fn in runs.items():
            try:
                timing, chunks = fn()
                row["methods"][name] = {**score(it["gold"], chunks), **{a: round(b, 1) for a, b in timing.items()}}
            except Exception as e:
                row["methods"][name] = {"error": str(e)[:300]}
        results.append(row)
        print(f"[{k}/{len(items)}] {it['id']} " + " ".join(f"{a}={min((r for r in v.get('gold_ranks', {}).values() if r), default='-')}" for a, v in row["methods"].items()), flush=True)
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    json.dump({"results": results}, open(out, "w"), indent=1)


# ---------- summary ----------

def acc(rows, method, k, mode):
    vals = []
    for r in rows:
        v = r["methods"].get(method)
        if not v or "error" in v:
            continue
        g = list(v["gold_ranks"].values())
        ok = [x is not None and x <= k for x in g]
        vals.append(all(ok) if mode == "all" else any(ok))
    return (100 * sum(vals) / len(vals), len(vals)) if vals else (float("nan"), 0)


def chunk_hit(rows, method, k):
    vals = []
    for r in rows:
        v = r["methods"].get(method)
        if not v or "error" in v or v.get("chunk_rank") == "n/a":
            continue
        vals.append(v["chunk_rank"] is not None and v["chunk_rank"] <= k)
    return 100 * sum(vals) / len(vals) if vals else float("nan")


def med(rows, method, key):
    xs = [r["methods"][method][key] for r in rows if method in r["methods"] and key in r["methods"][method]]
    return statistics.median(xs) if xs else float("nan")


def methods_of(rows):
    found = {m for r in rows for m, v in r["methods"].items() if "error" not in v}
    return [m for m in METHODS if m in found] + sorted(found - set(METHODS))


def tables(rows, run_note):
    used = methods_of(rows)
    n = len(rows)
    out = [f"{run_note} {n} instances scored.\n"]
    out.append("| Method | Acc@1 | Acc@5 | Acc@10 | Hit@1 | Hit@5 | Hit@10 | Chunk@5 | Chunk@10 | index, median | query, median |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for mth in used:
        a = [acc(rows, mth, k, "all")[0] for k in KS]
        h = [acc(rows, mth, k, "any")[0] for k in KS]
        c = [chunk_hit(rows, mth, k) for k in (5, 10)]
        cs = [f"{x:.1f}" if not math.isnan(x) else "" for x in c]
        out.append(f"| {LABELS.get(mth, mth)} | " + " | ".join(f"{x:.1f}" for x in a + h) + f" | {cs[0]} | {cs[1]} | {med(rows, mth, 'index_ms') / 1000:.1f} s | {med(rows, mth, 'query_ms'):.0f} ms |")
    out.append("\nAcc@10 by repository (every gold file in the top 10 files):\n")
    repos = sorted({r["repo"] for r in rows})
    out.append("| Repository | instances | " + " | ".join(LABELS.get(m, m) for m in used) + " | repomap index, median | repomap query, median |")
    out.append("|---|---:|" + "---:|" * len(used) + "---:|---:|")
    for repo in repos:
        sub = [r for r in rows if r["repo"] == repo]
        out.append(f"| {repo} | {len(sub)} | " + " | ".join(f"{acc(sub, m, 10, 'all')[0]:.1f}" for m in used) + f" | {med(sub, 'repomap', 'index_ms') / 1000:.1f} s | {med(sub, 'repomap', 'query_ms'):.0f} ms |")
    return "\n".join(out)


def summarize(manifest_path, out, shards, docs, note):
    rows = []
    for s in shards:
        rows += json.load(open(s))["results"]
    rows = [r for r in rows if "error" not in r]
    rows.sort(key=lambda r: r["id"])
    m = json.load(open(manifest_path))
    summary = {}
    for mth in methods_of(rows):
        summary[mth] = {
            **{f"acc@{k}": round(acc(rows, mth, k, "all")[0], 2) for k in KS},
            **{f"hit@{k}": round(acc(rows, mth, k, "any")[0], 2) for k in KS},
            "chunk@5": None if math.isnan(chunk_hit(rows, mth, 5)) else round(chunk_hit(rows, mth, 5), 2), "chunk@10": None if math.isnan(chunk_hit(rows, mth, 10)) else round(chunk_hit(rows, mth, 10), 2),
            "scored": acc(rows, mth, 1, "all")[1],
            "errors": sum(1 for r in rows if "error" in r["methods"].get(mth, {})),
            "index_ms_median": round(med(rows, mth, "index_ms")),
            "query_ms_median": round(med(rows, mth, "query_ms"), 1),
        }
    json.dump({"dataset": m["dataset"], "revision": m["revision"], "summary": summary, "results": rows}, open(out, "w"), separators=(",", ":"))
    text = tables(rows, note)
    print(text)
    if docs:
        d = open(docs).read()
        new = re.sub(r"(<!-- LOC:START -->\n).*?(\n<!-- LOC:END -->)", lambda mm: mm.group(1) + text + mm.group(2), d, flags=re.S)
        open(docs, "w").write(new)


if __name__ == "__main__":
    a = sys.argv[1:]
    if a[0] == "prepare":
        prepare(a[1], a[a.index("--split") + 1] if "--split" in a else "verified")
    elif a[0] == "run":
        shard = (0, 1)
        if "--shard" in a:
            i, n = a[a.index("--shard") + 1].split("/")
            shard = (int(i), int(n))
        only = a[a.index("--only") + 1:] if "--only" in a else []
        only = [x for x in only if not x.startswith("--")]
        skip = set()
        if "--no-semble" in a:
            skip.add("semble")
        if "--skip" in a:
            skip |= set(a[a.index("--skip") + 1].split(","))
        baseline = a[a.index("--baseline") + 1] if "--baseline" in a else None
        variants = json.load(open(a[a.index("--variants") + 1])) if "--variants" in a else {}
        run(a[1], a[2], a[3], a[4], shard, "--smoke" in a, only, skip, baseline, variants)
    elif a[0] == "summarize":
        docs = a[a.index("--docs") + 1] if "--docs" in a else None
        note = a[a.index("--note") + 1] if "--note" in a else ""
        skip = {docs, note, "--docs", "--note"}
        summarize(a[1], a[2], [x for x in a[3:] if x not in skip], docs, note)
