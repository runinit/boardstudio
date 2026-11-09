"""Prepare authenticated oracle inputs in a private, temporary directory.

This is not a fixture generator or a replacement for the Node-based gate.
"""

import argparse
import base64
from contextlib import contextmanager
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tarfile
import tempfile


ROOT = Path(__file__).resolve().parents[2]
PIN = "8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37"
VERSION = "4.2.1"
LOCK = Path(__file__).with_name("package-lock.json")
PACKAGE = Path(__file__).with_name("package.json")


def release(value):
    match = re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", value) if isinstance(value, str) else None
    if not match or any(int(part) > 2**53 - 1 for part in match.groups()):
        raise ValueError(f"Unsupported npm version: {value!r}")
    return tuple(map(int, match.groups()))


def satisfies(version, spec):
    # shortcut: only exact, caret and * release ranges; add reviewed npm semantics for other forms.
    found = release(version)
    if spec == "*":
        return True
    if not isinstance(spec, str):
        raise ValueError(f"Unsupported npm range: {spec!r}")
    if spec.startswith("^"):
        low = release(spec[1:])
        bump = next((i for i, part in enumerate(low) if part), 2)
        upper = low[:bump] + (low[bump] + 1,) + (0,) * (2 - bump)
        return low <= found < upper
    return found == release(spec)


def check_lock(lock, manifest):
    if (not isinstance(manifest, dict) or set(manifest) - {"name", "private", "dependencies"}
            or not isinstance(lock, dict) or lock.get("lockfileVersion") != 3
            or set(lock) - {"name", "lockfileVersion", "requires", "packages"}):
        raise ValueError("Unsupported manifest or lockfile format")
    packages = lock.get("packages")
    if not isinstance(packages, dict) or not isinstance(packages.get(""), dict):
        raise ValueError("Lockfile is missing root dependencies")
    root = packages[""]
    if (set(root) - {"name", "dependencies"} or not isinstance(manifest.get("dependencies"), dict)
            or not manifest["dependencies"] or root.get("dependencies") != manifest["dependencies"]):
        raise ValueError("Lockfile is missing required direct dependencies")
    fields = {"version", "resolved", "integrity", "license", "engines", "funding", "dependencies", "bin"}
    for name, metadata in packages.items():
        if not name:
            continue
        parts = name.split("/")
        if (len(parts) not in (2, 3) or parts[0] != "node_modules" or not all(parts)
                or any(part in (".", "..") for part in parts)
                or (len(parts) == 2 and parts[1].startswith("@"))
                or (len(parts) == 3 and (not parts[1].startswith("@") or parts[1] == "@"))
                or not all(re.fullmatch(r"[a-z0-9][a-z0-9._-]*", part) for part in
                           (parts[-1], parts[1][1:] if len(parts) == 3 else parts[1]))):
            raise ValueError(f"Unsupported lockfile package path: {name}")
        if not isinstance(metadata, dict) or set(metadata) - fields:
            raise ValueError(f"Unsupported lockfile package fields: {name}")
        version = metadata.get("version")
        release(version)
        registry = f"https://registry.npmjs.org/{'/'.join(parts[1:])}/-/{parts[-1]}-{version}.tgz"
        if metadata.get("resolved") != registry:
            raise ValueError(f"Unsupported package source: {name}")
    pending, reached = [""], set()
    while pending:
        name = pending.pop()
        if name in reached:
            continue
        reached.add(name)
        dependencies = packages[name].get("dependencies", {})
        if not isinstance(dependencies, dict):
            raise ValueError(f"Invalid dependencies for {name}")
        for dependency, spec in dependencies.items():
            child = "node_modules/" + dependency
            if child not in packages:
                raise ValueError(f"Lockfile missing dependency {dependency} required by {name}")
            version = packages[child].get("version")
            if not satisfies(version, spec):
                raise ValueError(f"{name} requires {dependency}@{spec}; locked {version}")
            pending.append(child)
    if reached != set(packages):
        raise ValueError("Lockfile contains unreachable dependencies")


def git(checkout, *args):
    # Do not let inherited Git environment redirect the checkout or replace objects.
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_TERMINAL_PROMPT="0")
    return subprocess.check_output(
        ["git", "--no-optional-locks", "--no-replace-objects", "-c", "core.hooksPath=/dev/null",
         "-c", "core.fsmonitor=false", "-C", str(checkout), *args],
        env=env, stderr=subprocess.PIPE,
    )


def extract(data, destination, strip_root=False):
    prefix = None
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:*") as archive:
        for member in archive:
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts or not path.parts:
                raise ValueError(f"Unsafe archive path: {member.name}")
            if not (member.isdir() or member.isfile()):
                raise ValueError(f"Unsupported archive member: {member.name}")
            if strip_root:
                if prefix is None:
                    prefix = path.parts[0]
                if path.parts[0] != prefix:
                    raise ValueError(f"Unexpected archive root: {member.name}")
                path = PurePosixPath(*path.parts[1:])
                if not path.parts:
                    if not member.isdir():
                        raise ValueError(f"Invalid archive root: {member.name}")
                    continue
                if "node_modules" in path.parts:
                    raise ValueError(f"Nested node_modules in package archive: {member.name}")
            target = destination.joinpath(*path.parts)
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                with target.open("xb") as output:
                    output.write(archive.extractfile(member).read())


@contextmanager
def verified_runtime(checkout, npm_cache, temp_dir=None):
    """Yield a private source/modules tree; cleanup on success or failure."""
    checkout = Path(checkout).resolve(strict=True)
    if checkout == ROOT or ROOT in checkout.parents:
        raise ValueError("Pinned checkout must be outside the repository")
    top = Path(os.fsdecode(git(checkout, "rev-parse", "--show-toplevel").strip())).resolve()
    if top != checkout:
        raise ValueError("Cache must name the pinned checkout root")
    if git(checkout, "rev-parse", "HEAD").strip().decode("ascii") != PIN:
        raise ValueError("Cache is not the pinned Ergogen commit")
    config = git(checkout, "config", "--null", "--name-only", "--list").split(b"\0")
    if any(key.lower().startswith(b"filter.") for key in config):
        raise ValueError("Pinned checkout config contains executable filter settings")
    index = git(checkout, "ls-files", "-v", "-z").split(b"\0")
    if any(entry and (entry[:1].islower() or entry[:1] == b"S") for entry in index):
        raise ValueError("Pinned checkout has hidden index changes")
    if git(checkout, "status", "--porcelain", "--untracked-files=all").strip():
        raise ValueError("Pinned checkout has tracked modifications or untracked files")

    # Archive the pinned Git object, not mutable checkout files or installed modules.
    source = git(checkout, "archive", "--format=tar", PIN)
    tree = git(checkout, "ls-tree", "-r", "-z", "--full-tree", PIN)
    lock = json.loads(LOCK.read_text(encoding="utf-8"))
    check_lock(lock, json.loads(PACKAGE.read_text(encoding="utf-8")))
    parent = Path(temp_dir if temp_dir is not None else
                  os.environ.get("TMPDIR") or os.environ.get("TEMP") or os.environ.get("TMP") or "/tmp").resolve(strict=True)
    if any(parent == protected or protected in parent.parents for protected in (ROOT, checkout)):
        raise ValueError("Temporary directory must be outside repository and pinned checkout")
    with tempfile.TemporaryDirectory(prefix="ergogen-verified-", dir=parent) as directory:
        root = Path(directory)
        upstream = root / "upstream"
        upstream.mkdir()
        extract(source, upstream)
        expected = {}
        for entry in tree.split(b"\0"):
            if not entry:
                continue
            metadata, name = entry.split(b"\t", 1)
            mode, kind, oid = metadata.split(b" ")
            if mode not in (b"100644", b"100755") or kind != b"blob":
                raise ValueError("Unsupported pinned source tree entry")
            expected[os.fsdecode(name)] = oid.decode("ascii")
        files = {file.relative_to(upstream).as_posix(): file for file in upstream.rglob("*") if file.is_file()}
        if files.keys() != expected.keys():
            raise ValueError("Pinned source archive differs from Git tree")
        for name, file in files.items():
            data = file.read_bytes()
            blob = b"blob " + str(len(data)).encode("ascii") + b"\0" + data
            if hashlib.sha1(blob).hexdigest() != expected[name]:
                raise ValueError(f"Pinned source blob mismatch: {name}")
        if json.loads((upstream / "package.json").read_text(encoding="utf-8"))["version"] != VERSION:
            raise ValueError("Pinned checkout package version does not match")
        for name, metadata in lock["packages"].items():
            if not name:
                continue
            parts = PurePosixPath(name).parts
            if len(parts) not in (2, 3) or parts[0] != "node_modules" or any(p in (".", "..") for p in parts):
                raise ValueError(f"Unsupported lockfile package path: {name}")
            if len(parts) == 3 and not parts[1].startswith("@"):
                raise ValueError(f"Unsupported lockfile package path: {name}")
            integrity = metadata["integrity"]
            if not integrity.startswith("sha512-"):
                raise ValueError(f"Unsupported integrity for {name}")
            digest = base64.b64decode(integrity[7:], validate=True)
            if len(digest) != 64:
                raise ValueError(f"Invalid integrity for {name}")
            hex_digest = digest.hex()
            data = (Path(npm_cache) / hex_digest[:2] / hex_digest[2:4] / hex_digest[4:]).read_bytes()
            if hashlib.sha512(data).digest() != digest:
                raise ValueError(f"Archive integrity mismatch: {name}")
            target = root.joinpath(*parts)
            target.mkdir(parents=True, exist_ok=False)
            extract(data, target, strip_root=True)
            package = json.loads((target / "package.json").read_text(encoding="utf-8"))
            if package.get("name") != "/".join(parts[1:]):
                raise ValueError(f"Package name mismatch: {name}")
            if package["version"] != metadata["version"]:
                raise ValueError(f"Package version mismatch: {name}")
            if package.get("dependencies", {}) != metadata.get("dependencies", {}):
                raise ValueError(f"Package dependency mismatch: {name}")
            if any(package.get(field) for field in ("optionalDependencies", "peerDependencies", "bundleDependencies", "bundledDependencies")):
                raise ValueError(f"Unsupported optional, peer or bundled dependencies: {name}")
        yield root


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checkout", help="existing clean, pinned external checkout")
    parser.add_argument("--npm-cache", type=Path, default=Path.home() / ".npm/_cacache/content-v2/sha512")
    args = parser.parse_args()
    with verified_runtime(args.checkout, args.npm_cache) as root:
        print(f"Verified pinned source and locked archives in private temporary tree: {root}")


if __name__ == "__main__":
    main()
