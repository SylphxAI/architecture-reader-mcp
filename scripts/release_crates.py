#!/usr/bin/env python3
"""Pack or publish the workspace crates using its existing Cargo version.

A temporary source tree adds registry constraints to path-only dependencies;
tracked manifests and the existing version setter remain unchanged.
"""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
USER_AGENT = "repomap-release (github.com/SylphxAI/repomap)"


def publish_order(metadata):
    packages = {p["name"]: p for p in metadata["packages"] if p["id"] in metadata["workspace_members"]}
    ordered, active, done = [], set(), set()

    def visit(name):
        if name in active:
            raise ValueError(f"workspace dependency cycle at {name}")
        if name in done:
            return
        active.add(name)
        for dep in packages[name]["dependencies"]:
            if dep["name"] in packages and dep.get("path") and dep.get("kind") != "dev":
                visit(dep["name"])
        active.remove(name)
        done.add(name)
        if packages[name].get("publish") != []:
            ordered.append(packages[name])

    for name in sorted(packages):
        visit(name)
    return ordered


def registry_constraints(text, package, packages):
    """Use Cargo metadata, not another version table, for internal edges."""
    for dep in package["dependencies"]:
        if not dep.get("path") or dep.get("kind") == "dev":
            continue
        target = packages.get(dep["name"])
        if target is None:
            raise ValueError(f"path dependency {dep['name']} is not a publishable workspace crate")
        key = dep.get("rename") or dep["name"]
        pattern = rf"(?m)^({re.escape(key)}\s*=\s*\{{)([^\n}}]*)(\}})"
        matches = list(re.finditer(pattern, text))
        if len(matches) != 1:
            raise ValueError(f"expected one inline dependency for {key}")
        match = matches[0]
        fields = match[2]
        version = f' version = "={target["version"]}",'
        if re.search(r"\bversion\s*=", fields):
            fields = re.sub(r'\bversion\s*=\s*"[^"]+"', f'version = "={target["version"]}"', fields)
        else:
            fields = version + fields
        text = text[:match.start()] + match[1] + fields + match[3] + text[match.end():]
    return text


def stage_sources(root, stage, metadata, files):
    packages = {p["name"]: p for p in publish_order(metadata)}
    directories = [Path(p["manifest_path"]).relative_to(root).parent for p in packages.values()]
    for name in files:
        path = Path(name)
        if name not in {"Cargo.toml", "Cargo.lock", "LICENSE", "README.md"} and not any(path.is_relative_to(d) for d in directories):
            continue
        target = stage / path
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(root / path, target)
    for package in packages.values():
        manifest = stage / Path(package["manifest_path"]).relative_to(root)
        manifest.write_text(registry_constraints(manifest.read_text(), package, packages))


def indexed(name, version):
    prefix = "1" if len(name) == 1 else "2" if len(name) == 2 else f"3/{name[0]}" if len(name) == 3 else f"{name[:2]}/{name[2:4]}"
    request = urllib.request.Request(f"https://index.crates.io/{prefix}/{name}", headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            rows = [json.loads(line) for line in response.read().decode().splitlines() if line]
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return False
        raise
    for row in rows:
        if row["vers"] == version:
            if row.get("yanked"):
                raise ValueError(f"{name} {version} is yanked; do not republish it")
            return True
    return False


def wait_for_index(name, version):
    deadline = time.monotonic() + 600
    while time.monotonic() < deadline:
        if indexed(name, version):
            return
        time.sleep(120)
    raise RuntimeError(f"{name} {version} did not become visible in the sparse index")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["list", "plan", "check", "publish"])
    args = parser.parse_args()
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"], cwd=ROOT))
    packages = publish_order(metadata)
    if not packages:
        raise ValueError("no publishable workspace crates")
    if args.mode == "list":
        for package in packages:
            print(package["name"], package["version"])
        return
    if args.mode == "plan":
        missing = [p["name"] for p in packages if not indexed(p["name"], p["version"])]
        print(json.dumps({"missing": missing, "order": [p["name"] for p in packages]}))
        if os.environ.get("GITHUB_OUTPUT"):
            with open(os.environ["GITHUB_OUTPUT"], "a") as output:
                output.write(f"publish={str(bool(missing)).lower()}\n")
        return
    target = ROOT / "target"
    target.mkdir(exist_ok=True)
    files = subprocess.check_output(["git", "-C", str(ROOT), "ls-files", "-z"]).decode().split("\0")
    with tempfile.TemporaryDirectory(prefix="registry-sources-", dir=target) as directory:
        stage = Path(directory)
        stage_sources(ROOT, stage, metadata, [f for f in files if f])
        selections = [value for p in packages for value in ["-p", p["name"]]]
        if args.mode == "check":
            subprocess.run(["cargo", "package", "--locked", "--registry", "crates-io", *selections], cwd=stage, check=True)
            build_root = Path(os.environ.get("CARGO_TARGET_DIR", stage / "target"))
            if not build_root.is_absolute():
                build_root = stage / build_root
            packed = build_root / "package"
            destination = target / "registry-packages"
            destination.mkdir(exist_ok=True)
            for package in packages:
                name = f'{package["name"]}-{package["version"]}.crate'
                shutil.copy2(packed / name, destination / name)
            return
        for package in packages:
            name, version = package["name"], package["version"]
            if indexed(name, version):
                print(f"{name} {version} already published; skipping")
                continue
            print(f"Publishing {name} {version}", flush=True)
            subprocess.run(["cargo", "publish", "--locked", "-p", name], cwd=stage, check=True)
            wait_for_index(name, version)


if __name__ == "__main__":
    main()
