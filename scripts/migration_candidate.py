"""Derive and atomically publish a proof for a completed migration build."""

from datetime import datetime
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import tempfile


BUILD_ID_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9-]*\Z")
WARNING_RE = re.compile(r"\bwarning(?:\[[^\]]+\])?:")
HEADERS = {
    "Cross-Origin-Opener-Policy": "same-origin",
    "Cross-Origin-Embedder-Policy": "require-corp",
}
PROGRESS_PATH = Path(__file__).resolve().parents[1] / ".scratch/dioxus-frontend-v1/progress.py"


def _load_progress():
    spec = importlib.util.spec_from_file_location("migration_candidate_progress", PROGRESS_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _load_inventory():
    path = Path(__file__).with_name("build-m1.py")
    spec = importlib.util.spec_from_file_location("migration_candidate_build_m1", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.sources


def _relative_path(value, label):
    path = Path(value)
    if path.is_absolute() or ".." in path.parts:
        raise ValueError(f"{label} must be a repository-relative path")
    return path


def _route_log(provenance, route, prefix, build_dir):
    matches = []
    for command in provenance["commands"]:
        argv = command.get("argv")
        if not isinstance(argv, list):
            continue
        try:
            index = argv.index("--base-path")
            command_prefix = argv[index + 1]
        except (ValueError, IndexError):
            continue
        if command_prefix == prefix and "dx" in argv and "build" in argv:
            matches.append(command)
    if len(matches) != 1:
        raise ValueError(f"Candidate provenance must identify one {route} release command")
    log_value = matches[0].get("log")
    if not isinstance(log_value, str) or not log_value:
        raise ValueError(f"Candidate {route} release command has no log record")
    log_path = Path(log_value)
    if not log_path.is_absolute():
        log_path = build_dir / log_path
    try:
        resolved = log_path.resolve(strict=True)
        resolved.relative_to(build_dir.resolve())
    except (OSError, ValueError) as error:
        raise ValueError(f"Candidate {route} release log is missing or outside its build: {log_value}") from error
    return len(WARNING_RE.findall(resolved.read_text(errors="replace")))


def _duration(commands):
    timestamps = []
    try:
        for command in commands:
            timestamps.extend(datetime.fromisoformat(command[key]) for key in ("started", "finished"))
    except (KeyError, TypeError, ValueError) as error:
        raise ValueError("Candidate provenance is missing valid command timestamps") from error
    if not timestamps:
        raise ValueError("Candidate provenance has no timestamped build commands")
    start, finish = min(timestamps), max(timestamps)
    seconds = (finish - start).total_seconds()
    if seconds < 0:
        raise ValueError("Candidate provenance has an invalid command duration")
    return seconds


def derive_proof(root, build_id, provenance_path, provenance_bytes, provenance, *, inventory):
    if provenance.get("build_id") != build_id:
        raise ValueError("Requested build id does not match provenance")
    if provenance.get("status") != "complete":
        raise ValueError("Candidate provenance is not complete")
    sources = provenance.get("sources")
    if not isinstance(sources, dict) or not sources:
        raise ValueError("Candidate provenance has no source inventory")
    current = inventory()
    if current != sources:
        paths = sorted(path for path in set(current) | set(sources) if current.get(path) != sources.get(path))
        raise ValueError("Candidate source inventory drift: " + ", ".join(paths[:12]))

    fresh = provenance.get("commands")
    if not isinstance(fresh, list) or not fresh or any(not isinstance(row, dict) for row in fresh):
        raise ValueError("Candidate provenance has no executed commands")
    lineage_fields = {"inherited_full_build_commands", "inherited_full_build_lineage"}
    reuse_markers = {"reuse_mode", "base_build", "base_source_commit", "base_provenance",
                     "base_provenance_sha256"}
    if lineage_fields <= provenance.keys():
        lineage = provenance["inherited_full_build_lineage"]
        inherited = provenance["inherited_full_build_commands"]
    elif not (lineage_fields & provenance.keys()) and not (reuse_markers & provenance.keys()):
        lineage, inherited = [], 0
    else:
        raise ValueError("Candidate provenance has incomplete inherited command lineage")
    if (not isinstance(lineage, list) or any(not isinstance(row, dict) for row in lineage)
            or type(inherited) is not int or inherited != len(lineage)):
        raise ValueError("Candidate provenance has invalid inherited command lineage")
    if any(row.get("exit", row.get("exit_code")) != 0 for row in [*fresh, *lineage]):
        raise ValueError("Candidate provenance contains a failed command")

    routes = {}
    warnings = {}
    for name in ("root", "subpath"):
        route = provenance.get(name)
        if not isinstance(route, dict) or not isinstance(route.get("assets"), dict):
            raise ValueError(f"Candidate provenance has no {name} asset manifest")
        if not route["assets"]:
            raise ValueError(f"Candidate provenance has an empty {name} asset manifest")
        routes[name] = {
            "http": 200,
            "headers": dict(HEADERS),
            "asset_count": len(route["assets"]),
            "mismatches": [],
        }
        warnings[name] = _route_log(provenance, name, route.get("prefix"), root / "web/target/builds" / build_id)

    return {
        "build_id": build_id,
        "source_commit": provenance["source_commit"],
        "provenance": provenance_path.as_posix(),
        "provenance_sha256": hashlib.sha256(provenance_bytes).hexdigest(),
        "source_count": len(sources),
        "source_mismatches": [],
        "routes": routes,
        "fresh_commands": len(fresh),
        "inherited_commands": inherited,
        "duration_seconds": _duration(fresh),
        "release_warnings": warnings,
        "qualification": (
            "Combined package provenance, current source hashes and local assets verified; "
            "live asset checks performed by record-candidate. Functional journey evidence is separate."
        ),
    }


def _atomic_bytes(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    mode = path.stat().st_mode if path.exists() else 0o644
    temporary = None
    try:
        with tempfile.NamedTemporaryFile("wb", dir=path.parent, prefix=f".{path.name}.",
                                         suffix=".tmp", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        os.chmod(temporary, mode)
        os.replace(temporary, path)
        temporary = None
        directory = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def publish(root, build_id, root_url, subpath_url, proof_relative=None, *, inventory=None,
            progress_module=None):
    root = Path(root).resolve()
    if not BUILD_ID_RE.fullmatch(build_id):
        raise ValueError("Build id must be alphanumeric with optional hyphens")
    progress = progress_module or _load_progress()
    inventory = inventory or _load_inventory()
    provenance_relative = Path("web/target/builds") / build_id / "provenance.json"
    provenance_path = progress.repository_file(provenance_relative, "provenance")
    provenance_bytes = provenance_path.read_bytes()
    try:
        provenance = json.loads(provenance_bytes)
    except json.JSONDecodeError as error:
        raise ValueError(f"Malformed candidate provenance: {error}") from error
    if not isinstance(provenance, dict):
        raise ValueError("Malformed candidate provenance")
    try:
        proof = derive_proof(root, build_id, provenance_relative, provenance_bytes, provenance,
                             inventory=inventory)
    except (KeyError, TypeError) as error:
        raise ValueError(f"Malformed candidate provenance; missing {error.args[0]}") from error

    proof_relative = _relative_path(
        proof_relative or Path(".scratch/dioxus-frontend-v1/evidence") / build_id / "package-proof.json",
        "proof path",
    )
    proof_target = root / proof_relative
    try:
        proof_target.resolve().relative_to(root)
    except ValueError as error:
        raise ValueError("Proof path must resolve within the repository") from error
    proof_bytes = (json.dumps(proof, indent=2) + "\n").encode()
    previous_proof = proof_target.read_bytes() if proof_target.exists() else None
    if previous_proof is not None and previous_proof != proof_bytes:
        raise ValueError(f"Refusing to replace a different finalized proof: {proof_relative}")

    # Validate local provenance and package bytes before changing either
    # canonical record. record_candidate owns the single live route verification.
    proof_target.parent.mkdir(parents=True, exist_ok=True)
    temp_proof = proof_target.parent / f".{proof_target.name}.validate-{os.getpid()}"
    try:
        temp_proof.write_bytes(proof_bytes)
        progress.validate_provenance(temp_proof)
    finally:
        temp_proof.unlink(missing_ok=True)

    run_path = root / progress.RUN
    previous_run = run_path.read_bytes()
    _atomic_bytes(proof_target, proof_bytes)
    try:
        # Reuse the canonical writer and all of its route, record, and
        # qualification logic. Restore both files if validation fails.
        changed = progress.record_candidate(proof_relative.as_posix(), root_url, subpath_url)
    except BaseException:
        if previous_proof is None:
            proof_target.unlink(missing_ok=True)
        else:
            _atomic_bytes(proof_target, previous_proof)
        if run_path.read_bytes() != previous_run:
            _atomic_bytes(run_path, previous_run)
        raise
    return changed or previous_proof is None
