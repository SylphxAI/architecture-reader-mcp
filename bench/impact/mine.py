#!/usr/bin/env python3
"""Mine impact gold cases from the history of a public repository.

usage: mine.py <repo-dir> <language> <pin-sha> [--max N] [--scan N]

Walks the first-parent history back from <pin-sha> and prints candidate cases
as JSON. The gold rule is mechanical and does not use repomap:

  commit   non-merge, 2 to 12 changed files, at least one test file, at least
           one source file besides the seed.
  seed     a changed, non-test source file that defines identifiers (see
           DEFS) the other changed files use.
  callers  other changed non-test source files whose ADDED lines contain, as a
           whole word, an identifier the seed defines. These are call sites
           the commit itself had to change.
  tests    changed test files whose added lines contain such an identifier, or
           the seed's module name.
  callees  the reverse: files the seed's added lines reference through an
           identifier they define, that the commit also changed. Recorded for
           a future trace benchmark; `impact` reports upstream only.

Identifiers are at least 5 characters, unique to one changed file in the
commit, and not on a stop list of generic names. Cases are taken in history
order from the pin, so the choice does not depend on any tool's output.
"""
import json
import re
import subprocess
import sys

EXT = {
    "rust": (".rs",),
    "typescript": (".ts", ".tsx"),
    "python": (".py",),
    "go": (".go",),
}
DEFS = {
    "rust": r"\b(?:fn|struct|enum|trait|type|const|static|mod)\s+([A-Za-z_][A-Za-z0-9_]*)",
    "typescript": r"\b(?:function|class|interface|type|enum|const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)",
    "python": r"^\s*(?:def|class|async def)\s+([A-Za-z_][A-Za-z0-9_]*)",
    "go": r"\b(?:func(?:\s*\([^)]*\))?|type|const|var)\s+([A-Za-z_][A-Za-z0-9_]*)",
}
STOP = {
    "tests", "test", "error", "result", "value", "values", "string", "options", "config", "context", "request",
    "response", "default", "return", "format", "output", "input", "state", "build", "start", "close", "write",
    "parse", "handle", "equal", "assert", "should", "Error", "Result", "String", "Option", "Self", "self",
    "false", "true", "which", "before", "after", "other", "match", "const", "static", "async", "await",
    "main", "init", "setup", "teardown", "setUp", "tearDown", "mock", "fixture", "client", "server",
}


def git(repo, *a):
    return subprocess.run(["git", "-C", repo, *a], capture_output=True, text=True, check=True).stdout


def is_test(path):
    p = path.lower()
    name = p.rsplit("/", 1)[-1]
    return (
        any(s in ("test", "tests", "__tests__", "spec", "specs", "testdata", "e2e") for s in p.split("/"))
        or name.startswith("test_")
        or ".test." in name
        or ".spec." in name
        or "_test." in name
    )


def added_lines(repo, sha, path):
    d = git(repo, "diff", "--unified=0", "--no-color", f"{sha}^", sha, "--", path)
    return "\n".join(l[1:] for l in d.splitlines() if l.startswith("+") and not l.startswith("+++"))


def defs(repo, sha, path, lang):
    try:
        text = git(repo, "show", f"{sha}:{path}")
    except subprocess.CalledProcessError:
        return set()
    return {m for m in re.findall(DEFS[lang], text, re.M) if len(m) >= 5 and m not in STOP and not m.startswith("test")}


def has_word(text, w):
    return re.search(r"(?<![A-Za-z0-9_$])" + re.escape(w) + r"(?![A-Za-z0-9_$])", text) is not None


def case(repo, sha, lang):
    files = git(repo, "diff", "--name-only", "--diff-filter=AMR", f"{sha}^", sha).split()
    if not 2 <= len(files) <= 12:
        return None
    exts = EXT[lang]
    src = [f for f in files if f.endswith(exts) and not is_test(f)]
    tst = [f for f in files if f.endswith(exts) and is_test(f)]
    if len(src) < 2 or not tst:
        return None
    all_defs = {f: defs(repo, sha, f, lang) for f in src}
    added = {f: added_lines(repo, sha, f) for f in src + tst}
    best = None
    for seed in src:
        mine = {i for i in all_defs[seed] if not any(i in all_defs[o] for o in src if o != seed)}
        if not mine:
            continue
        module = re.sub(r"\.[a-z]+$", "", seed.rsplit("/", 1)[-1])
        callers = [f for f in src if f != seed and any(has_word(added[f], i) for i in mine)]
        tests = [f for f in tst if any(has_word(added[f], i) for i in mine) or (len(module) >= 4 and has_word(added[f], module))]
        callees = [f for f in src if f != seed and any(has_word(added[seed], i) for i in all_defs[f] if not any(i in all_defs[o] for o in src if o != f))]
        if (callers or callees) and tests:
            score = len(callers) + len(tests)
            if best is None or score > best[0]:
                best = (score, {"seed": seed, "callers": callers, "callees": callees, "tests": tests})
    if best is None:
        return None
    subject = git(repo, "log", "-1", "--format=%s", sha).strip()
    return {"commit": sha, "subject": subject, **best[1]}


def main():
    repo, lang, pin = sys.argv[1:4]
    mx = int(sys.argv[sys.argv.index("--max") + 1]) if "--max" in sys.argv else 6
    scan = int(sys.argv[sys.argv.index("--scan") + 1]) if "--scan" in sys.argv else 400
    out = []
    for sha in git(repo, "rev-list", "--first-parent", "--no-merges", f"--max-count={scan}", pin).split():
        try:
            c = case(repo, sha, lang)
        except subprocess.CalledProcessError:
            continue
        if c:
            out.append(c)
            if len(out) >= mx:
                break
    json.dump(out, sys.stdout, indent=2)
    print()


main()
