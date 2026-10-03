#!/usr/bin/env python3
"""Impact benchmark: precision and recall of `repomap impact --json` at file level.

usage: run.py <repomap-binary> <corpus-dir> <out.json> [--repo NAME ...] [--floor floor.json]
       run.py --summary <out.json>

For every case in gold.json the repository is checked out at the commit,
`repomap impact <seed-file> --json` runs on it, and the answer is compared with
the gold lists (see README.md for how they were derived):

  callers  predicted = files + importers of the answer, minus the seed and minus tests
  tests    predicted = tests of the answer, minus the seed

Precision and recall are per case, then averaged per language, then over cases
(micro over languages is not used: Rust has 12 of 30 cases). A case with an
empty gold list is left out of that axis. An empty prediction scores precision 0.
Exits 1 when a --floor file is given and a metric is below it.
"""
import json
import os
import subprocess
import sys
import tempfile
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))


def git(repo, *a):
    return subprocess.run(["git", "-C", repo, *a], check=True, capture_output=True, text=True).stdout


def prf(pred, gold):
    pred, gold = set(pred), set(gold)
    hit = len(pred & gold)
    return (hit / len(pred) if pred else 0.0), hit / len(gold)


def clone(spec, corpus):
    dest = os.path.join(corpus, spec["name"])
    if not os.path.isdir(os.path.join(dest, ".git")):
        subprocess.run(["git", "clone", "-q", "--filter=blob:none", "--no-checkout", spec["url"], dest], check=True)
    git(dest, "cat-file", "-e", spec["pin"] + "^{commit}")
    return dest


def predict(binary, root, seed, cache):
    env = dict(os.environ, REPOMAP_CACHE_DIR=cache, REPOMAP_EMBED="0")
    p = subprocess.run([binary, "impact", seed, "--json", "--root", root], capture_output=True, text=True, env=env, timeout=600)
    if p.returncode != 0:
        return None, p.stderr.strip()[-300:]
    r = json.loads(p.stdout)
    tests = {t for t in r.get("tests", []) if t != seed}
    callers = ({*r.get("files", []), *r.get("importers", [])} - {seed}) - tests
    return {"callers": sorted(callers), "tests": sorted(tests), "risk": r.get("risk")}, None


def run(binary, corpus, out, only):
    gold = json.load(open(os.path.join(HERE, "gold.json")))
    specs = {r["name"]: r for r in gold["repos"]}
    results = []
    cache = tempfile.mkdtemp(prefix="impact-cache-")
    for spec in gold["repos"]:
        cases = [c for c in gold["cases"] if c["repo"] == spec["name"]]
        if not cases or (only and spec["name"] not in only):
            continue
        root = clone(spec, corpus)
        for c in cases:
            git(root, "checkout", "-q", "-f", c["commit"])
            pred, err = predict(binary, root, c["seed"], cache)
            row = {"id": c["id"], "repo": c["repo"], "language": spec["language"], "seed": c["seed"], "error": err}
            for axis in ("callers", "tests"):
                got = pred[axis] if pred else []
                row[axis + "_pred"] = got
                row[axis + "_gold"] = c[axis]
                if c[axis]:
                    row[axis + "_p"], row[axis + "_r"] = prf(got, c[axis])
            gold_all = set(c["callers"]) | set(c["tests"])
            if gold_all and pred:
                row["all_p"], row["all_r"] = prf(set(pred["callers"]) | set(pred["tests"]), gold_all)
            elif gold_all:
                row["all_p"], row["all_r"] = 0.0, 0.0
            results.append(row)
            print(f"{c['id']}: {err or 'ok'}", file=sys.stderr)
    summary = summarise(results)
    json.dump({"summary": summary, "cases": results}, open(out, "w"), indent=1)
    return summary


def mean(xs):
    return sum(xs) / len(xs) if xs else None


def summarise(results):
    s = {}
    for axis in ("callers", "tests", "all"):
        for m in ("p", "r"):
            key = f"{axis}_{m}"
            by_lang = defaultdict(list)
            for r in results:
                if key in r:
                    by_lang[r["language"]].append(r[key])
            s[key] = {"macro_over_languages": mean([mean(v) for v in by_lang.values()]), "by_language": {k: mean(v) for k, v in sorted(by_lang.items())}, "cases": sum(len(v) for v in by_lang.values())}
    s["errors"] = sum(1 for r in results if r["error"])
    return s


def markdown(summary):
    f = lambda v: "n/a" if v is None else f"{v * 100:.1f}%"
    langs = sorted({k for v in summary.values() if isinstance(v, dict) for k in v.get("by_language", {})})
    lines = ["| Axis | Metric | Overall | " + " | ".join(langs) + " | Cases |", "|---|---|---:|" + "---:|" * len(langs) + "---:|"]
    names = {"callers": "Callers", "tests": "Tests", "all": "Callers + tests"}
    for axis in ("callers", "tests", "all"):
        for m, mn in (("p", "Precision"), ("r", "Recall")):
            d = summary[f"{axis}_{m}"]
            lines.append(f"| {names[axis]} | {mn} | {f(d['macro_over_languages'])} | " + " | ".join(f(d["by_language"].get(l)) for l in langs) + f" | {d['cases']} |")
    lines.append(f"\nErrors (no answer): {summary['errors']}")
    return "\n".join(lines)


def check_floor(summary, floor_path):
    floor = json.load(open(floor_path))
    bad = []
    for k, v in floor.items():
        if k == "max_errors":
            continue
        got = summary[k]["macro_over_languages"]
        if got is None or got < v:
            bad.append(f"{k}: {got} < floor {v}")
    if summary["errors"] > floor.get("max_errors", 0):
        bad.append(f"errors: {summary['errors']}")
    return bad


if __name__ == "__main__":
    if sys.argv[1] == "--summary":
        print(markdown(json.load(open(sys.argv[2]))["summary"]))
        sys.exit(0)
    binary, corpus, out = sys.argv[1:4]
    only = [a for i, a in enumerate(sys.argv) if i > 0 and sys.argv[i - 1] == "--repo"]
    summary = run(binary, corpus, out, only)
    print(markdown(summary))
    if "--floor" in sys.argv:
        bad = check_floor(summary, sys.argv[sys.argv.index("--floor") + 1])
        if bad:
            print("BELOW FLOOR:\n" + "\n".join(bad))
            sys.exit(1)
