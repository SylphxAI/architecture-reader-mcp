import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import urllib.error

import release_crates as release


def metadata(root):
    core = {"id": "core", "name": "repomap-core", "version": "1.5.0", "publish": None,
            "manifest_path": str(root / "crates/repomap-core/Cargo.toml"), "dependencies": []}
    binary = {"id": "binary", "name": "repomap", "version": "1.5.0", "publish": None,
              "manifest_path": str(root / "crates/repomap/Cargo.toml"),
              "dependencies": [{"name": "repomap-core", "path": str(root / "crates/repomap-core"), "kind": None}]}
    return {"workspace_members": ["binary", "core"], "packages": [binary, core]}


class ReleaseCratesTests(unittest.TestCase):
    def test_dependency_order_and_cycle(self):
        data = metadata(release.ROOT)
        self.assertEqual([p["name"] for p in release.publish_order(data)], ["repomap-core", "repomap"])
        data["packages"][1]["dependencies"] = [{"name": "repomap", "path": "binary", "kind": None}]
        with self.assertRaisesRegex(ValueError, "cycle"):
            release.publish_order(data)

    def test_staging_adds_current_constraint_without_changing_source(self):
        target = release.ROOT / "target"
        target.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=target) as directory:
            root = Path(directory) / "source"
            stage = Path(directory) / "stage"
            data = metadata(root)
            files = {"Cargo.toml": "[workspace]\n", "Cargo.lock": "version = 4\n", "LICENSE": "MIT\n",
                     "crates/repomap-core/Cargo.toml": '[package]\nname = "repomap-core"\n',
                     "crates/repomap/Cargo.toml": '[dependencies]\nrepomap-core = { path = "../repomap-core" }\n',
                     "crates/repomap/assets/app.js": "asset", "docs/unused.md": "not packaged"}
            for name, text in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text)
            release.stage_sources(root, stage, data, list(files))
            self.assertIn('version = "=1.5.0"', (stage / "crates/repomap/Cargo.toml").read_text())
            self.assertEqual((root / "crates/repomap/Cargo.toml").read_text(), files["crates/repomap/Cargo.toml"])
            self.assertEqual((stage / "crates/repomap/assets/app.js").read_text(), "asset")
            self.assertFalse((stage / "docs/unused.md").exists())

    def test_constraints_follow_metadata_version_and_reject_unknown_paths(self):
        data = metadata(release.ROOT)
        binary, core = data["packages"]
        core["version"] = "2.0.1"
        text = 'repomap-core = { path = "../repomap-core", version = "1" }\n'
        self.assertIn('version = "=2.0.1"', release.registry_constraints(text, binary, {core["name"]: core}))
        with self.assertRaisesRegex(ValueError, "publishable workspace"):
            release.registry_constraints(text, binary, {})

    def test_sparse_index_and_yanked_versions(self):
        row = {"vers": "1.5.0", "yanked": False}
        with patch.object(release.urllib.request, "urlopen", return_value=io.BytesIO(json.dumps(row).encode())):
            self.assertTrue(release.indexed("repomap", "1.5.0"))
        row["yanked"] = True
        with patch.object(release.urllib.request, "urlopen", return_value=io.BytesIO(json.dumps(row).encode())):
            with self.assertRaisesRegex(ValueError, "yanked"):
                release.indexed("repomap", "1.5.0")
        error = urllib.error.HTTPError("index", 404, "missing", {}, None)
        with patch.object(release.urllib.request, "urlopen", side_effect=error):
            self.assertFalse(release.indexed("repomap", "1.5.0"))
        error = urllib.error.HTTPError("index", 503, "unavailable", {}, None)
        with patch.object(release.urllib.request, "urlopen", side_effect=error):
            with self.assertRaises(urllib.error.HTTPError):
                release.indexed("repomap", "1.5.0")


if __name__ == "__main__":
    unittest.main()
