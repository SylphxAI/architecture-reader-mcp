"""Offline benchmark validity checks: no binaries, inference, models or network."""
import ast
import contextlib
import copy
import importlib.util
import io
import json
import math
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("localize", ROOT / "bench/localization/localize.py")
loc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(loc)


def mcp_class(path):
    # The latency/search scripts run their CLI at import time. Load only the
    # actual class, without invoking any producer or changing its implementation.
    tree = ast.parse((ROOT / path).read_text())
    cls = next(n for n in tree.body if isinstance(n, ast.ClassDef) and n.name == "Mcp")
    ns = {"time": time, "json": json, "K": 10}
    exec(compile(ast.Module(body=[cls], type_ignores=[]), str(ROOT / path), "exec"), ns)
    return ns["Mcp"]


class ValidityTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.manifest = {"dataset": "offline", "revision": "frozen", "smoke": ["a"],
                         "items": [{"id": x, "repo": "fixture/repo", "gold": {"a.py": [[1, 2]]}}
                                   for x in ("a", "b", "c")]}
        self.manifest["items"].append({"id": "new-file-only", "gold": {}})
        self.mf = self.root / "manifest.json"
        self.mf.write_text(json.dumps(self.manifest))
        self.out = self.root / "results.json"
        self.docs = self.root / "docs.md"
        self.docs.write_text("<!-- LOC:START -->\noriginal\n<!-- LOC:END -->")
        self.good = {"gold_ranks": {"a.py": 1}, "chunk_rank": 1, "index_ms": 10, "query_ms": 1}

    def shards(self, count=2, smoke=False, only=(), skip=("keywords", "bm25", "semble")):
        ids = [i["id"] for i in loc.selection(self.manifest, smoke, only)]
        size = math.ceil(len(ids) / count)
        methods = loc.planned_methods(skip)
        return [{"plan": {"dataset": "offline", "revision": "frozen", "ids": ids,
                          "methods": methods, "shard": n, "shards": count},
                 "results": [{"id": x, "repo": "fixture/repo", "methods": {m: copy.deepcopy(self.good) for m in methods}}
                             for x in ids[n * size:(n + 1) * size]]} for n in range(count)]

    def merge(self, data, count=2, **kwargs):
        paths = []
        for n, shard in enumerate(data):
            path = self.root / f"shard-{n}.json"
            path.write_text(json.dumps(shard))
            paths.append(str(path))
        with contextlib.redirect_stdout(io.StringIO()):
            loc.summarize(str(self.mf), str(self.out), paths, str(self.docs), "offline",
                          skip=kwargs.pop("skip", ("keywords", "bm25", "semble")), shard_count=count, **kwargs)
        return json.loads(self.out.read_text())

    def test_complete_shards_and_intentionally_unselected(self):
        result = self.merge(self.shards())
        self.assertEqual(result["summary"]["repomap"]["scored"], 3)
        self.assertEqual(result["summary"]["repomap"]["acc@10"], 100)
        self.assertEqual(set(result["summary"]), {"repomap"})
        result = self.merge(self.shards(1, smoke=True), count=1, smoke=True)
        self.assertEqual(result["summary"]["repomap"]["scored"], 1)
        result = self.merge(self.shards(1, only=["b"]), count=1, only=["b"])
        self.assertEqual([r["id"] for r in result["results"]], ["b"])

    def test_all_methods_and_bm25_na(self):
        data = self.shards(1, skip=())
        for row in data[0]["results"]:
            row["methods"]["bm25"]["chunk_rank"] = "n/a"
        result = self.merge(data, count=1, skip=())
        self.assertEqual(result["summary"]["bm25"]["scored"], 3)
        self.assertIsNone(result["summary"]["bm25"]["chunk@10"])
        self.assertEqual(loc.planned_methods({"keywords", "bm25", "semble"}, "before", {"v": "tune"}),
                         ["repomap", "main", "repomap:v"])

    def test_invalid_coverage_never_writes_summary_or_docs(self):
        cases = []
        data = self.shards(); cases.append(data[:1])
        data = self.shards(); cases.append(data + [copy.deepcopy(data[0])])
        for change in ("missing-row", "duplicate-row", "checkout", "missing-method", "error-method",
                       "unplanned-method", "missing-gold", "changed-plan", "legacy", "false-na"):
            data = self.shards()
            row = data[0]["results"][0]
            if change == "missing-row": data[0]["results"].pop()
            elif change == "duplicate-row": data[0]["results"].append(copy.deepcopy(row))
            elif change == "checkout": row["error"] = "checkout failed"
            elif change == "missing-method": row["methods"] = {}
            elif change == "error-method": row["methods"]["repomap"] = {"error": "index failed"}
            elif change == "unplanned-method": row["methods"]["main"] = self.good
            elif change == "missing-gold": row["methods"]["repomap"]["gold_ranks"] = {}
            elif change == "changed-plan": data[0]["plan"]["ids"] = ["a"]
            elif change == "legacy": del data[0]["plan"]
            elif change == "false-na": row["methods"]["repomap"]["chunk_rank"] = "n/a"
            cases.append(data)
        for data in cases:
            with self.subTest(data=data):
                with self.assertRaises(ValueError): self.merge(data)
                self.assertFalse(self.out.exists())
                self.assertIn("original", self.docs.read_text())

    def test_failed_missing_methods_do_not_shrink_denominator(self):
        rows = self.shards(1)[0]["results"]
        rows[1]["methods"]["repomap"] = {"error": "failed"}
        rows[2]["methods"] = {}
        for metric in (lambda: loc.acc(rows, "repomap", 10, "all"),
                       lambda: loc.chunk_hit(rows, "repomap", 10)):
            with self.assertRaises(ValueError): metric()

    def test_empty_hits_are_misses_and_bm25_is_not_applicable(self):
        empty = loc.score({"a.py": [[1, 2]]}, [])
        row = {"id": "a", "methods": {"repomap": empty}}
        self.assertEqual(loc.acc([row], "repomap", 10, "all"), (0, 1))
        self.assertEqual(loc.chunk_hit([row], "repomap", 10), 0)
        self.assertEqual(loc.score({"a.py": [[1, 2]]}, [], False)["chunk_rank"], "n/a")

    def test_run_freezes_plan_before_checkout_or_method_failure(self):
        dataset = [{"instance_id": x, "problem_statement": "offline"} for x in ("a", "b", "c")]
        with patch.object(loc, "load_dataset", return_value=dataset), patch.object(loc, "checkout", side_effect=RuntimeError("offline checkout failure")):
            loc.run("unused", str(self.mf), str(self.root / "work"), str(self.out), (0, 1), False, [],
                    {"keywords", "bm25", "semble"}, None, {})
        data = json.loads(self.out.read_text())
        self.assertEqual(data["plan"]["ids"], ["a", "b", "c"])
        self.assertEqual(len(data["results"]), 3)
        self.out.unlink()
        with self.assertRaises(ValueError): self.merge([data], count=1)

    def test_rpc_and_tool_errors_rejected_but_empty_hits_preserved(self):
        for path, method in (("scripts/bench.py", "tool"), ("scripts/bench_search.py", "search"),
                             ("bench/localization/localize.py", "search")):
            cls = mcp_class(path)
            obj = cls.__new__(cls)
            for envelope in ({"error": {"code": -32603, "message": "offline"}},
                             {"result": {"isError": True, "content": [{"text": "offline"}]}}):
                with self.subTest(path=path, envelope=envelope):
                    obj.call_raw = obj.call = lambda *a: envelope
                    with self.assertRaises(RuntimeError):
                        obj.tool("search", {}) if method == "tool" else obj.search("offline")
            obj.call_raw = obj.call = lambda *a: {"result": {"content": [{"text": '{"hits": []}'}]}}
            result = obj.tool("search", {}) if method == "tool" else obj.search("offline")
            self.assertEqual(result[1]["content"][0]["text"] if method == "tool" else result[1],
                             '{"hits": []}' if method == "tool" else [])
            # Exercise the real transport loop, including initialize errors.
            obj = cls.__new__(cls)
            obj.id = 0
            obj.p = type("Producer", (), {"stdin": io.StringIO(), "stdout": io.StringIO(
                json.dumps({"id": 1, "error": {"code": -32603}}) + "\n")})()
            with self.assertRaises(RuntimeError):
                (obj.call_raw if method == "tool" else obj.call)("initialize", {})

    def test_workflow_pipeline_preserves_producer_failure(self):
        workflow = (ROOT / ".github/workflows/bench.yml").read_text()
        import re
        bodies = re.findall(r"shell: bash\n        run: \|\n(.*?)(?=\n      -)", workflow, re.S)
        self.assertEqual(len(bodies), 2)
        for body in bodies:
            # Replay the checked-in shell with an inert Python producer which
            # prints a plausible partial result then exits nonzero.
            body = "\n".join(line[10:] for line in body.splitlines())
            body = re.sub(r"python3 scripts/bench.py[^|]+", "python3 -c 'print(\"partial benchmark\"); raise SystemExit(7)' ", body)
            result = subprocess.run(["bash", "-c", body], env={"GITHUB_STEP_SUMMARY": str(self.root / "summary")},
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 7)
            self.assertIn("partial benchmark", result.stdout)


if __name__ == "__main__":
    unittest.main()
