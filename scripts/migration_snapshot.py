"""Committed build inputs and a reusable, isolated migration build checkout.

The coordinator's checkout may continue changing after its commit is selected.
Only this owned checkout and its generated files are used by a snapshot build.
"""

from contextlib import contextmanager
import fcntl
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import posixpath
import re
import subprocess


SCHEMA = 1
SNAPSHOT_DIRECTORY = "web/target/migration-snapshot"
CACHE_DIRECTORIES = ("release", "wasm32-unknown-unknown/release")
CRATE_DIRECTORIES = ("web", "core", "renderer", "cad/wasm")


def git(root, *arguments):
    return subprocess.check_output(
        ["git", "-C", str(root), *arguments], stderr=subprocess.PIPE,
    ).decode().strip()


def resolve_commit(root, revision):
    commit = git(root, "rev-parse", "--verify", f"{revision}^{{commit}}")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("snapshot source must resolve to a Git commit")
    return commit


def manifest_digest(sources):
    return hashlib.sha256(json.dumps(sources, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def committed_sources(root, revision, source_path):
    """Hash the complete maintained input set from Git, including internal symlinks."""
    commit = resolve_commit(root, revision)
    raw = subprocess.check_output(["git", "-C", str(root), "ls-tree", "-r", "-z", commit])
    entries = {}
    for row in raw.split(b"\0"):
        if not row:
            continue
        metadata, path = row.split(b"\t", 1)
        mode, kind, oid = metadata.decode().split()
        entries[os.fsdecode(path)] = (mode, kind, oid)
    process = subprocess.Popen(
        ["git", "-C", str(root), "cat-file", "--batch"],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    hashes, links = {}, {}

    def read_blob(oid, *, link=False):
        cache = links if link else hashes
        if oid in cache:
            return cache[oid]
        process.stdin.write((oid + "\n").encode())
        process.stdin.flush()
        header = process.stdout.readline().decode().strip().split()
        if len(header) != 3 or header[0] != oid or header[1] != "blob":
            raise ValueError("snapshot input is not an available Git blob")
        size = int(header[2])
        if link and size > 4096:
            raise ValueError("snapshot source symlink target is too long")
        digest, chunks, remaining = hashlib.sha256(), [], size
        while remaining:
            data = process.stdout.read(min(remaining, 1024 * 1024))
            if not data:
                raise ValueError("incomplete Git blob while hashing snapshot source")
            digest.update(data)
            if link:
                chunks.append(data)
            remaining -= len(data)
        if process.stdout.read(1) != b"\n":
            raise ValueError("malformed Git blob stream")
        cache[oid] = b"".join(chunks).decode() if link else digest.hexdigest()
        return cache[oid]

    def content_hash(name, seen=()):
        if name in seen or len(seen) > 20:
            raise ValueError(f"recursive source symlink in snapshot: {name}")
        entry = entries.get(name)
        if entry is None or entry[1] != "blob":
            raise ValueError(f"snapshot source is absent or not a regular input: {name}")
        mode, _, oid = entry
        if mode == "120000":
            target = read_blob(oid, link=True)
            if not target or PurePosixPath(target).is_absolute() or "\\" in target:
                raise ValueError(f"snapshot source symlink escapes repository: {name}")
            resolved = posixpath.normpath(posixpath.join(posixpath.dirname(name), target))
            if resolved == ".." or resolved.startswith("../"):
                raise ValueError(f"snapshot source symlink escapes repository: {name}")
            return content_hash(resolved, (*seen, name))
        if mode not in {"100644", "100755"}:
            raise ValueError(f"unsupported snapshot input mode: {name}")
        return read_blob(oid)

    try:
        return {name: content_hash(name) for name in sorted(entries) if source_path(name)}
    finally:
        process.stdin.close()
        process.stdout.close()
        process.stderr.close()
        process.wait()


def source_identity(root, sources, source_path, revision="HEAD"):
    if not isinstance(sources, dict) or not sources:
        raise ValueError("snapshot inputs differ from their committed source: missing manifest")
    commit = resolve_commit(root, revision)
    expected = committed_sources(root, commit, source_path)
    if sources != expected:
        changed = sorted(name for name in set(sources) | set(expected)
                         if sources.get(name) != expected.get(name))
        raise ValueError("snapshot inputs differ from their committed source: " + ", ".join(changed[:12]))
    return {"schema": SCHEMA, "source_commit": commit,
            "source_tree": git(root, "rev-parse", f"{commit}^{{tree}}"),
            "manifest_sha256": manifest_digest(sources)}


def validate_identity(root, provenance, source_path):
    """Validate a frozen package independently of newer files in the coordinator."""
    identity = provenance.get("source_snapshot")
    if not isinstance(identity, dict) or identity.get("schema") != SCHEMA:
        raise ValueError("candidate snapshot identity is missing or unsupported")
    commit = provenance.get("source_commit")
    if not re.fullmatch(r"[0-9a-f]{40}", str(commit)) or identity.get("source_commit") != commit:
        raise ValueError("candidate snapshot commit does not match provenance")
    expected = source_identity(root, provenance.get("sources"), source_path, commit)
    if identity != expected:
        raise ValueError("candidate snapshot tree or manifest identity differs")
    return expected


@contextmanager
def package_slot(root):
    """One package process per Git repository; unrelated source commits remain free."""
    common = Path(git(root, "rev-parse", "--git-common-dir"))
    if not common.is_absolute():
        common = (Path(root) / common).resolve()
    with (common / "migration-package.lock").open("a+") as stream:
        try:
            fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise ValueError("another migration package is active in this repository") from error
        try:
            yield
        finally:
            fcntl.flock(stream, fcntl.LOCK_UN)


def _seed_release_cache(root, checkout):
    for crate in CRATE_DIRECTORIES:
        for relative in CACHE_DIRECTORIES:
            source = root / crate / "target" / relative
            destination = checkout / crate / "target" / relative
            if source.is_dir() and not destination.exists():
                destination.parent.mkdir(parents=True, exist_ok=True)
                subprocess.run(["cp", "-a", "--reflink=auto", str(source), str(destination)], check=True)


def prepare_checkout(root, revision, *, prepare_dependencies=True, seed_cache=True):
    """Create/update only the owned detached build checkout, preserving warm outputs."""
    root = Path(root).resolve()
    commit = resolve_commit(root, revision)
    checkout = root / SNAPSHOT_DIRECTORY
    marker = root / "web/target/migration-snapshot-owner.json"
    identity = {"schema": SCHEMA, "coordinator": str(root), "checkout": str(checkout)}
    if checkout.exists():
        if not marker.is_file() or json.loads(marker.read_text()) != identity:
            raise ValueError("existing snapshot checkout is not owned by this coordinator")
        if Path(git(checkout, "rev-parse", "--show-toplevel")).resolve() != checkout:
            raise ValueError("snapshot checkout is not the expected Git worktree")
        def common_directory(path):
            value = Path(git(path, "rev-parse", "--git-common-dir"))
            return (path / value).resolve() if not value.is_absolute() else value.resolve()
        if common_directory(checkout) != common_directory(root):
            raise ValueError("snapshot checkout belongs to another Git repository; preserving it")
        if git(checkout, "status", "--porcelain=v1", "--untracked-files=no"):
            raise ValueError("snapshot checkout has modified tracked files; preserving them")
        subprocess.run(["git", "-C", str(checkout), "checkout", "--detach", commit], check=True)
    else:
        checkout.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["git", "-C", str(root), "worktree", "add", "--detach", str(checkout), commit], check=True)
        marker.write_text(json.dumps(identity, indent=2) + "\n")
    if seed_cache:
        _seed_release_cache(root, checkout)
    if prepare_dependencies and (checkout / "pnpm-lock.yaml").is_file():
        subprocess.run(["pnpm", "install", "--offline", "--frozen-lockfile", "--ignore-scripts"],
                       cwd=checkout, check=True)
    return checkout
