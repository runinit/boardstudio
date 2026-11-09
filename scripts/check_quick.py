"""Changed-file selection for `python3 scripts/check.py quick`.

Maps the files you changed to the checks that cover them. A path the mapping cannot place
falls back to the full affected steps and is named in the output, so a gap is visible
rather than silently skipped.
"""

from __future__ import annotations

from dataclasses import dataclass, field
import json
from pathlib import Path
import subprocess

# Edits to these change how everything builds or runs, so every Rust check applies.
SHARED = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo/", "deny.toml")
# Tooling scripts are covered by the `tooling` step.
TOOLING = ("scripts/", "cad/scripts/")
# Prose and plans need only the documentation checks.
DOCS = ("docs/", "README.md", "AGENTS.md", "CLAUDE.md", "GLOSSARY.md", "DESIGN.md", "PRODUCT.md")
# Data and assets consumed through a build rather than compiled: the build step owns them.
DATA = ("web/assets/", "content/", "ergogen/", "catalogue/modules/", "contracts/")


@dataclass
class Package:
    name: str
    directory: str
    dependencies: set[str] = field(default_factory=set)


@dataclass
class Selection:
    tooling: bool = False
    rust: bool = False
    cad: bool = False
    full_rust: bool = False
    # Tooling scripts whose own suite runs in another step (not `tooling`).
    extra_tests: list[str] = field(default_factory=list)
    packages: set[str] = field(default_factory=set)
    page_files: list[str] = field(default_factory=list)
    unplaced: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)


def workspace_packages(root: Path) -> dict[str, Package]:
    out = subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
                         cwd=root, check=True, capture_output=True, text=True).stdout
    metadata = json.loads(out)
    members = {package["name"] for package in metadata["packages"]}
    packages = {}
    for package in metadata["packages"]:
        directory = str(Path(package["manifest_path"]).parent.relative_to(root))
        packages[package["name"]] = Package(
            package["name"], "" if directory == "." else directory,
            {dep["name"] for dep in package["dependencies"] if dep["name"] in members})
    return packages


def changed_files(root: Path, base: str = "main") -> list[str]:
    def git(*args: str) -> list[str]:
        done = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)
        return done.stdout.split("\n") if done.returncode == 0 else []

    merge_base = next((line for ref in (base, f"origin/{base}")
                       if (line := (git("merge-base", "HEAD", ref) or [""])[0])), "HEAD")
    files = set(git("diff", "--name-only", merge_base)) | set(git("ls-files", "--others", "--exclude-standard"))
    return sorted(file for file in files if file)


def owner(path: str, packages: dict[str, Package]) -> Package | None:
    """The workspace package whose directory is the longest prefix of the path."""
    best = None
    for package in packages.values():
        directory = package.directory
        if directory and (path == directory or path.startswith(directory + "/")):
            if best is None or len(directory) > len(best.directory):
                best = package
    return best


def dependents(names: set[str], packages: dict[str, Package]) -> set[str]:
    """The packages plus everything that depends on them, directly or not."""
    closure = set(names)
    changed = True
    while changed:
        changed = False
        for package in packages.values():
            if package.name not in closure and package.dependencies & closure:
                closure.add(package.name)
                changed = True
    return closure


def select(files: list[str], packages: dict[str, Package]) -> Selection:
    selection = Selection()
    direct: set[str] = set()
    for path in files:
        if path.startswith(DOCS):
            continue
        if path.startswith(SHARED):
            selection.full_rust = True
            selection.rust = True
            selection.notes.append(f"{path}: shared build input, every Rust check applies")
        elif path.startswith(TOOLING):
            selection.tooling = True
            if path == "scripts/test-import-kicad-parts.py":
                # Its suite moved to the `test` step; tooling alone would never run it.
                selection.extra_tests.append(path)
            if path in ("cad/scripts/test-cad-capture-controls.py", "cad/scripts/test-cold-responses.py",
                        "cad/scripts/measure-cad-worker.cjs", "scripts/model-lookup.js",
                        "scripts/model-lookup-pilot.py", "scripts/model-lookup-pilot.mjs",
                        "scripts/model-lookup-pilot-worker.js", "scripts/ergogen-parity/cases.cjs",
                        "scripts/ergogen-parity/generate.py", "scripts/ergogen-parity/test_generate.py"):
                selection.notes.append(f"{path}: run `check.py browser` to cover this browser control")
        elif path.startswith("cad/") and not path.startswith("cad/scripts/"):
            selection.cad = True
        elif path.startswith(DATA):
            selection.notes.append(f"{path}: data consumed by the build; run `check.py build` to cover it")
        elif (package := owner(path, packages)) is not None:
            selection.rust = True
            direct.add(package.name)
            if package.directory == "web" and path.startswith(("web/src/", "web/tests/")):
                if path.startswith("web/src/") and path.endswith(".rs"):
                    selection.page_files.append(path)
                else:
                    selection.full_rust = True
                    selection.notes.append(f"{path}: not a web/src Rust file; every Rust check applies")
        else:
            selection.unplaced.append(path)
            selection.full_rust = True
            selection.rust = True
    selection.packages = dependents(direct, packages)
    if selection.full_rust:
        selection.packages = set(packages)
    return selection
