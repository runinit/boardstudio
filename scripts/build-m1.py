#!/usr/bin/env python3
"""Build a uniquely staged M1 candidate and retain commands/source/asset hashes."""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import tomllib
from datetime import datetime, timezone

REPO = Path(__file__).resolve().parents[1]
WEB = REPO / "web"
BUILD_ROOT = WEB / "target" / "builds"

# Keep the historical audited leaf list explicit, then add only Rust modules
# proven page-only by the active page/provider feature graphs below. In
# particular, layout_align_geometry.rs has a core-worker test alias and stays
# explicitly outside the derived page-only set.
PAGE_ONLY_LEAF_PATHS = {
    "layout-command-pill": (
        "web/src/presentation/layout_workspace.rs",
        "web/src/presentation/objects/layout_toolbar.rs",
        "web/src/presentation/objects/layout_transform_toolbar.rs",
        "web/src/presentation/workspace_composition.rs",
    ),
    "pcb-part-input-inspector": (
        "web/src/presentation/pcb_wiring/part_input_settings.rs",
    ),
    "parts-definition-name-editor": (
        "web/src/parts_definition_name.rs",
    ),
}
PAGE_ONLY_ALLOWLIST = frozenset({
    "web/src/presentation/panels.rs",
    "web/src/presentation/panels_scroll_tests.rs",
    "web/assets/m1.css",
    *(path for paths in PAGE_ONLY_LEAF_PATHS.values() for path in paths),
})
# Cargo/build entry points remain byte-identical to the full baseline. The
# active module graph is resolved from those roots for each locked feature set;
# source additions qualify only when their resolved owners are page/test-only.
PAGE_ONLY_PROOF_PATHS = (
    "web/src/lib.rs",
    "web/Cargo.toml",
    "web/build.rs",
    "core/Cargo.toml",
)
PAGE_ONLY_MAIN_PATH = "web/src/main.rs"
REUSE_HELPER_PATH = "scripts/build-m1.py"
FIXTURE_PREPARATION_PATH = "scripts/prepare-m1-fixtures.mjs"
# The page-reuse guard changed in the full candidate at bbd4. Reuse remains
# compatible with that exact full-build helper only when the baseline source
# identity, source-manifest SHA-256, and Git blob identity all agree.
COMPATIBLE_FULL_BUILD_HELPERS = {
    (
        "bbd4da1b7cc0609dd4ae6d8ec0332031b0690ea1",
        "f350570e89f57ade0ba87d0a891d84826b318f056fab4da518826d4a750f705f",
    ): "157c6222db575eeec7d30be1e72e45ba49fb7a22",
}
CORE_TEST_ONLY_PATHS = frozenset({"core/tests/electrical_wiring.rs"})
# These standalone harnesses import this helper; candidate commands never execute
# them. Keep their bytes in source provenance and the final drift guard while
# treating their edits as verification-only, just like cfg(test) Rust inputs.
BUILD_TEST_ONLY_PATHS = frozenset({
    "scripts/test-build-m1-reuse.py",
    "scripts/test-build-m1-sources.py",
    "scripts/test-migration-deliver.py",
})
# The guard executes in the build CLI, so its exact source may change without
# changing packaged providers. Keep this separate from verification-only files.
BUILD_CONTROL_ONLY_PATHS = frozenset({"scripts/migration-deliver.py"})
NON_PAGE_RUST_ALIASES = frozenset({"web/src/presentation/objects/layout_align_geometry.rs"})
REUSED_PROVIDER_PREFIXES = (
    "assets/cad/",
    "assets/cad-worker/",
    "assets/core-worker/",
    "assets/renderer/",
    "assets/fixtures/",
    "assets/ergogen-models/",
)
FIXTURE_REFRESH_INHERITED_PREFIXES = tuple(
    prefix for prefix in REUSED_PROVIDER_PREFIXES if prefix != "assets/fixtures/"
) + ("assets/layout-generators", "assets/preview-generator/")

# Full builds produce these generated files. Their maintained source inputs are
# still hashed; generated copies are validated as packaged assets instead.
GENERATED_SOURCE_PATHS = (
    "web/target/",
    "core/pkg/",
    "cad/wasm/pkg/",
    "cad/.cache/",
    "renderer/pkg/",
    "web/assets/ergogen-models/",
    "web/assets/layout-generators/",
    "web/assets/preview-generator/",
    "web/assets/layout-generators.js",
)
ROOT_BUILD_INPUTS = frozenset({
    ".cargo/config",
    ".cargo/config.toml",
    ".node-version",
    ".npmrc",
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "rust-toolchain.toml",
})


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rust_lex(source):
    """Tokenize the Rust constructs used by the registration guard.

    Comments and whitespace do not affect the signature. String/raw-string
    literals stay atomic so words such as `mod` in UI copy cannot look like
    declarations. Nested block comments and balanced token groups are handled;
    malformed strings/comments and unmatched or mismatched token groups fail
    closed before any candidate output is created.
    """
    if isinstance(source, bytes):
        source = source.decode("utf-8")
    tokens = []
    index = 0
    length = len(source)

    def raw_string_end(start, prefix_length):
        quote = start + prefix_length
        hashes = 0
        while quote < length and source[quote] == "#":
            hashes += 1
            quote += 1
        if quote >= length or source[quote] != '"':
            return None
        terminator = '"' + ("#" * hashes)
        end = source.find(terminator, quote + 1)
        if end < 0:
            raise ValueError("unterminated Rust raw string in page leaf")
        return end + len(terminator)

    while index < length:
        char = source[index]
        if char.isspace():
            index += 1
            continue
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            index = length if end < 0 else end + 1
            continue
        if source.startswith("/*", index):
            depth = 1
            index += 2
            while index < length and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            if depth:
                raise ValueError("unterminated Rust block comment in page leaf")
            continue

        raw_prefix = next((prefix for prefix in ("br", "cr", "r") if source.startswith(prefix, index)), None)
        if raw_prefix is not None:
            raw_end = raw_string_end(index, len(raw_prefix))
            if raw_end is not None:
                tokens.append(("literal", source[index:raw_end]))
                index = raw_end
                continue
        if source.startswith("r#", index) and index + 2 < length and (source[index + 2].isalpha() or source[index + 2] == "_"):
            end = index + 3
            while end < length and (source[end].isalnum() or source[end] == "_"):
                end += 1
            tokens.append(("ident", source[index:end]))
            index = end
            continue

        literal_start = index
        quote_index = index
        if char in "bc" and index + 1 < length and source[index + 1] in "\"'":
            quote_index += 1
        if source[quote_index] == '"':
            index = quote_index + 1
            while index < length:
                if source[index] == "\\":
                    index += 2
                elif source[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            else:
                raise ValueError("unterminated Rust string in page leaf")
            tokens.append(("literal", source[literal_start:index]))
            continue
        if source[quote_index] == "'":
            # Distinguish a quoted character from a lifetime token. Character
            # contents are atomic; lifetime names cannot be registration words.
            end = quote_index + 1
            if end < length and source[end] == "\\":
                end += 2
            elif end < length:
                end += 1
            if end < length and source[end] == "'":
                index = end + 1
                tokens.append(("literal", source[literal_start:index]))
                continue
            if char in "bc":
                raise ValueError("malformed Rust byte character in page leaf")

        if char.isalpha() or char == "_":
            end = index + 1
            while end < length and (source[end].isalnum() or source[end] == "_"):
                end += 1
            tokens.append(("ident", source[index:end]))
            index = end
            continue
        if char.isdigit():
            end = index + 1
            while end < length and (source[end].isalnum() or source[end] in "_."):
                end += 1
            tokens.append(("number", source[index:end]))
            index = end
            continue
        tokens.append(("punct", char))
        index += 1
    return tokens


EXPECTED_PAGE_BUILD_CONFIGS = (
    frozenset({"page"}),
    frozenset({"core-worker"}),
    frozenset({"cad-worker"}),
    frozenset({"service-worker"}),
)


def page_build_feature_configs(repo_root):
    """Derive the release feature roots from Cargo and the full-build command matrix."""
    repo_root = Path(repo_root)
    manifest_path = repo_root / "web/Cargo.toml"
    try:
        manifest = tomllib.loads(manifest_path.read_text())
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise ValueError(f"cannot read web Cargo feature ownership: {error}") from error

    features = manifest.get("features")
    binaries = manifest.get("bin")
    if not isinstance(features, dict) or not isinstance(binaries, list):
        raise ValueError("web Cargo manifest has no explicit features/binary ownership")
    page_binary = [row for row in binaries if isinstance(row, dict) and row.get("name") == "boardstudio-web"]
    if len(page_binary) != 1 or page_binary[0].get("required-features") != ["page"]:
        raise ValueError("web binary is no longer gated only by the page feature")

    commands = expected_full_commands(Path("/page-feature-ownership-probe"))
    by_label = {label: argv for label, argv, _, _ in commands}
    expected_labels = {
        "page": ("page-root", "page-subpath"),
        "core-worker": ("core-worker",),
        "cad-worker": ("cad-worker",),
        "service-worker": ("offline-worker-root", "offline-worker-subpath"),
    }
    derived = []
    for feature, labels in expected_labels.items():
        if feature not in features:
            raise ValueError(f"web Cargo manifest does not declare the {feature} feature")
        observed = []
        for label in labels:
            argv = by_label.get(label)
            if argv is None or "--no-default-features" not in argv or "--features" not in argv:
                raise ValueError(f"full build command does not explicitly select {feature}: {label}")
            feature_index = argv.index("--features")
            if feature_index + 1 >= len(argv) or argv[feature_index + 1] != feature:
                raise ValueError(f"full build command feature differs from Cargo ownership: {label}")
            observed.append(frozenset({feature}))
        if any(item != observed[0] for item in observed):
            raise ValueError(f"root/subpath feature roots disagree for {feature}")
        derived.append(observed[0])

    configurations = tuple(derived)
    if configurations != EXPECTED_PAGE_BUILD_CONFIGS:
        raise ValueError("full build feature matrix differs from the reviewed page/provider ownership")

    # --features selects roots, not the complete cfg(feature) set. Cargo also
    # activates local feature dependencies and optional dependency features.
    dependency_tables = [manifest.get("dependencies", {})]
    dependency_tables.extend(
        target.get("dependencies", {}) for target in manifest.get("target", {}).values()
        if isinstance(target, dict)
    )
    optional = {
        name for table in dependency_tables for name, row in table.items()
        if isinstance(row, dict) and row.get("optional") is True
    }
    if any(not isinstance(items, list) or not all(isinstance(item, str) for item in items)
           for items in features.values()):
        raise ValueError("unsupported Cargo feature declarations; refusing provider reuse")
    namespaced = {item[4:] for items in features.values() for item in items if item.startswith("dep:")}
    local = dict(features)
    local.update({name: [f"dep:{name}"] for name in optional - namespaced if name not in local})

    def resolve(roots):
        enabled = set()
        pending = list(roots)
        while pending:
            name = pending.pop()
            if name in enabled:
                continue
            if name not in local:
                raise ValueError(f"unknown Cargo feature dependency: {name}")
            enabled.add(name)
            for item in local[name]:
                if item.startswith("dep:"):
                    if item[4:] not in optional:
                        raise ValueError(f"unknown optional Cargo dependency: {item}")
                elif "/" in item:
                    # Dependency feature/weak-feature activation needs Cargo's
                    # resolver. Unsupported shapes require the full build.
                    raise ValueError(f"unsupported Cargo dependency feature: {item}")
                else:
                    pending.append(item)
        return frozenset(enabled)

    return tuple(resolve(roots) for roots in configurations)


def _rust_group_end(values, start):
    pairs = {"[": "]", "{": "}", "(": ")"}
    if start >= len(values) or values[start] not in pairs:
        raise ValueError("malformed Rust module graph: expected a balanced group")
    stack = [pairs[values[start]]]
    index = start + 1
    while index < len(values) and stack:
        value = values[index]
        if value in pairs:
            stack.append(pairs[value])
        elif value in ("]", "}", ")"):
            if not stack or value != stack.pop():
                raise ValueError("malformed Rust module graph: mismatched group")
        index += 1
    if stack:
        raise ValueError("malformed Rust module graph: unterminated group")
    return index


def _rust_cfg_expression(values, features):
    """Evaluate the cfg atoms used by this crate for its locked wasm commands."""
    index = 0
    environment = {
        "target_arch": "wasm32",
        "target_os": "unknown",
        "target_env": "",
        "target_vendor": "unknown",
        "target_family": "wasm",
        "target_pointer_width": "32",
        "target_endian": "little",
        "test": "__test__" in features,
        "debug_assertions": False,
    }

    def parse_atom():
        nonlocal index
        if index >= len(values) or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", values[index]):
            raise ValueError("malformed Rust cfg expression in page module graph")
        name = values[index]
        index += 1
        if index < len(values) and values[index] == "(":
            index += 1
            parts = []
            while index < len(values) and values[index] != ")":
                parts.append(parse_atom())
                if index < len(values) and values[index] == ",":
                    index += 1
                elif index < len(values) and values[index] != ")":
                    raise ValueError("malformed Rust cfg list in page module graph")
            if index >= len(values) or values[index] != ")":
                raise ValueError("unterminated Rust cfg list in page module graph")
            index += 1
            if name == "all":
                return all(parts)
            if name == "any":
                return any(parts)
            if name == "not" and len(parts) == 1:
                return not parts[0]
            raise ValueError(f"unsupported Rust cfg operator in page module graph: {name}")
        if index < len(values) and values[index] == "=":
            index += 1
            if index >= len(values):
                raise ValueError("missing Rust cfg value in page module graph")
            raw = values[index]
            index += 1
            try:
                expected = json.loads(raw)
            except (json.JSONDecodeError, TypeError) as error:
                raise ValueError("unsupported Rust cfg value in page module graph") from error
            if name == "feature":
                return expected in features
            if name in environment:
                return environment[name] == expected
            raise ValueError(f"unknown Rust cfg key in page module graph: {name}")
        if name in environment and isinstance(environment[name], bool):
            return environment[name]
        raise ValueError(f"unknown bare Rust cfg in page module graph: {name}")

    result = parse_atom()
    if index != len(values):
        raise ValueError("trailing Rust cfg tokens in page module graph")
    return result


def _module_attributes(attributes, features):
    enabled = True
    explicit_path = None
    for attribute in attributes:
        values = [value for _, value in attribute]
        if len(values) < 3 or values[0] != "#" or values[1] not in ("[", "!"):
            continue
        start = 2 if values[1] == "[" else 3
        if values[1] == "!":
            if len(values) < 4 or values[2] != "[":
                raise ValueError("malformed inner Rust attribute in page module graph")
            start = 3
        if start >= len(values):
            raise ValueError("empty Rust attribute in page module graph")
        name = values[start]
        if name == "cfg_attr":
            raise ValueError("cfg_attr module ownership is unsupported; refusing provider reuse")
        if name == "cfg":
            if values[start + 1] != "(":
                raise ValueError("malformed cfg attribute in page module graph")
            expression_end = _rust_group_end(values, start + 1)
            expression = values[start + 2:expression_end - 1]
            enabled = enabled and _rust_cfg_expression(expression, features)
        elif name == "path":
            if values[start + 1:start + 2] != ["="] or start + 2 >= len(values):
                raise ValueError("malformed path attribute in page module graph")
            try:
                explicit_path = json.loads(values[start + 2])
            except (json.JSONDecodeError, TypeError) as error:
                raise ValueError("unsupported module path in page module graph") from error
    return enabled, explicit_path


def _module_base(path):
    if path.name in {"mod.rs", "main.rs", "lib.rs"}:
        return path.parent
    return path.parent / path.stem


def rust_module_graph(repo_root, root_relative, features, *, allow_opaque_macros=False):
    """Resolve supported Rust input edges; ambiguous registrations require full builds."""
    repo_root = Path(repo_root).resolve()
    web_root = (repo_root / "web").resolve()
    found = set()
    active = set()
    generated_includes = {
        "web/src/service_worker.rs": "/offline_manifest.rs",
        "web/src/bundled_models.rs": "/bundled_ergogen_models.rs",
    }

    def walk(module_path, included=False):
        module_path = module_path.resolve()
        try:
            module_path.relative_to(web_root)
        except ValueError as error:
            raise ValueError(f"Rust module escapes web/: {module_path}") from error
        relative = module_path.relative_to(repo_root).as_posix()
        if relative in active:
            raise ValueError(f"recursive Rust module graph: {relative}")
        if not module_path.is_file():
            raise ValueError(f"Rust module is missing from source graph: {relative}")
        tokens = rust_lex(module_path.read_bytes())
        # Included code inherits its call site's module context. Rather than
        # guess directory semantics, only module-free literal includes qualify.
        if included and any(kind == "ident" and value in {"mod", "macro", "macro_rules"}
                            for kind, value in tokens) and not allow_opaque_macros:
            raise ValueError(f"unsupported module registration in included Rust source: {relative}")
        if relative in found:
            return
        found.add(relative)
        active.add(relative)
        values = [value for _, value in tokens]
        for index, (kind, value) in enumerate(tokens):
            if kind != "ident" or value != "include" or values[index + 1:index + 2] != ["!"]:
                continue
            start = index + 2
            end = _rust_group_end(values, start)
            expression = values[start + 1:end - 1]
            if len(expression) == 1:
                try:
                    path = json.loads(expression[0])
                except json.JSONDecodeError as error:
                    raise ValueError("unsupported literal Rust include path") from error
                if not isinstance(path, str) or not path:
                    raise ValueError("unsupported literal Rust include path")
                walk(module_path.parent / path, included=True)
            else:
                expected = generated_includes.get(relative)
                allowed = ["concat", "!", "(", "env", "!", "(", '"OUT_DIR"', ")", ",",
                           json.dumps(expected), ")"] if expected else None
                if expression != allowed:
                    raise ValueError(f"unsupported dynamic Rust include in module graph: {relative}")
                # These exact generated tables are owned by the byte-pinned
                # web/build.rs and hashed generator inputs, and rebuilt fresh.
        walk_tokens(tokens, module_path, _module_base(module_path), relative)
        active.remove(relative)

    def walk_tokens(tokens, module_path, module_dir, relative):
        values = [value for _, value in tokens]
        index = 0
        pending_attributes = []
        while index < len(values):
            value = values[index]
            if value == ";":
                pending_attributes.clear()
                index += 1
                continue
            if value == "#" and index + 1 < len(values) and values[index + 1] in ("[", "!"):
                start = index + 1 + (values[index + 1] == "!")
                end = _rust_group_end(values, start)
                pending_attributes.append(tokens[index:end])
                index = end
                continue
            item = index
            if allow_opaque_macros and value == "macro_rules" and values[index + 1:index + 2] == ["!"]:
                group = index + 2
                while group < len(values) and values[group] != "{":
                    group += 1
                if group >= len(values):
                    raise ValueError(f"malformed macro_rules definition in {relative}")
                index = _rust_group_end(values, group)
                if index < len(values) and values[index] == ";":
                    index += 1
                pending_attributes.clear()
                continue
            if values[item] == "pub":
                item += 1
                if item < len(values) and values[item] == "(":
                    item = _rust_group_end(values, item)
            while item < len(values) and values[item] in ("unsafe", "default"):
                item += 1
            if item < len(values) and values[item] == "mod":
                if item + 2 >= len(values) or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", values[item + 1]):
                    raise ValueError(f"malformed module declaration in {relative}")
                name = values[item + 1]
                after_name = item + 2
                enabled, explicit_path = _module_attributes(pending_attributes, features)
                pending_attributes.clear()
                if values[after_name] == ";":
                    if enabled:
                        if explicit_path is not None:
                            path_base = module_path.parent if module_dir == _module_base(module_path) else module_dir
                            child = path_base / explicit_path
                        else:
                            options = [module_dir / f"{name}.rs", module_dir / name / "mod.rs"]
                            existing = [path for path in options if path.is_file()]
                            if len(existing) != 1:
                                raise ValueError(f"ambiguous or missing Rust module {name} declared by {relative}")
                            child = existing[0]
                        walk(child)
                    index = after_name + 1
                    continue
                if values[after_name] == "{":
                    end = _rust_group_end(values, after_name)
                    if enabled:
                        if explicit_path is not None:
                            raise ValueError(f"unsupported path on inline Rust module: {relative}")
                        walk_tokens(tokens[after_name + 1:end - 1], module_path, module_dir / name, relative)
                    index = end
                    continue
                raise ValueError(f"unsupported Rust module item in {relative}")
            pending_attributes.clear()
            if value in {"macro", "macro_rules"}:
                raise ValueError(f"unsupported macro registration in Rust module graph: {relative}")
            # Macro expansion cannot be treated as proof of no source edges.
            macro = item
            while macro + 2 < len(values) and values[macro + 1:macro + 3] == [":", ":"]:
                macro += 3
            if macro + 1 < len(values) and values[macro + 1] == "!":
                if values[macro] not in {"include", "thread_local", "wasm_bindgen_test_configure"}:
                    if not allow_opaque_macros:
                        raise ValueError(f"unsupported item macro in Rust module graph: {relative}")
                    group = macro + 2
                    while group < len(values) and values[group] not in ("(", "[", "{"):
                        group += 1
                    if group >= len(values):
                        raise ValueError(f"malformed macro invocation in {relative}")
                    index = _rust_group_end(values, group)
                    if index < len(values) and values[index] == ";":
                        index += 1
                    pending_attributes.clear()
                    continue
            cursor = index
            while cursor < len(values):
                if values[cursor] in ("[", "(", "{"):
                    end = _rust_group_end(values, cursor)
                    if (not allow_opaque_macros and
                            any(kind == "ident" and token in {"mod", "macro", "macro_rules"}
                                for kind, token in tokens[cursor + 1:end - 1])):
                        raise ValueError(f"unsupported nested registration in Rust module graph: {relative}")
                    closing = values[end - 1]
                    cursor = end
                    if closing == "}":
                        break
                    continue
                if values[cursor] == ";":
                    cursor += 1
                    break
                cursor += 1
            if cursor <= index:
                raise ValueError(f"could not advance Rust module graph at {relative}")
            index = cursor

    walk(repo_root / root_relative)
    return frozenset(found)


def page_only_rust_paths(repo_root):
    """Return Rust files compiled only by the fresh page command, not providers."""
    configurations = page_build_feature_configs(repo_root)
    page_graph = set(rust_module_graph(repo_root, "web/src/main.rs", configurations[0]))
    page_graph.update(rust_module_graph(repo_root, "web/src/lib.rs", configurations[0]))
    provider_graph = set()
    for features in configurations[1:]:
        provider_graph.update(rust_module_graph(repo_root, "web/src/lib.rs", features))
    return frozenset(page_graph - provider_graph - NON_PAGE_RUST_ALIASES)


def page_test_only_rust_paths(repo_root):
    """Return separate web test modules excluded from every release command."""
    configurations = page_build_feature_configs(repo_root)
    release_graph = set(rust_module_graph(repo_root, "web/src/main.rs", configurations[0]))
    release_graph.update(rust_module_graph(repo_root, "web/src/lib.rs", configurations[0]))
    for features in configurations[1:]:
        release_graph.update(rust_module_graph(repo_root, "web/src/lib.rs", features))
    test_graph = set(rust_module_graph(repo_root, "web/src/lib.rs", configurations[0] | {"__test__"}))
    test_graph.update(rust_module_graph(repo_root, "web/src/main.rs", configurations[0] | {"__test__"}))
    test_graph.update(
        path
        for root in standalone_web_test_roots(repo_root)
        for path in rust_module_graph(
            repo_root, root, configurations[0] | {"__test__"}, allow_opaque_macros=True
        )
    )
    return frozenset((test_graph - release_graph) - NON_PAGE_RUST_ALIASES)


def standalone_web_test_roots(repo_root):
    """Return Cargo's top-level web integration-test crate roots."""
    repo_root = Path(repo_root).resolve()
    tests_dir = repo_root / "web/tests"
    return tuple(path.relative_to(repo_root).as_posix() for path in sorted(tests_dir.glob("*.rs")))


def page_feature_ownership(repo_root):
    configurations = page_build_feature_configs(repo_root)
    page_graph = set(rust_module_graph(repo_root, "web/src/main.rs", configurations[0]))
    page_graph.update(rust_module_graph(repo_root, "web/src/lib.rs", configurations[0]))
    provider_graphs = {}
    for name, features in zip(("core-worker", "cad-worker", "service-worker"), configurations[1:]):
        provider_graphs[name] = set(rust_module_graph(repo_root, "web/src/lib.rs", features))
    providers = set().union(*provider_graphs.values()) if provider_graphs else set()
    test_graph = set(rust_module_graph(repo_root, "web/src/lib.rs", configurations[0] | {"__test__"}))
    test_graph.update(rust_module_graph(repo_root, "web/src/main.rs", configurations[0] | {"__test__"}))
    test_graph.update(
        path
        for root in standalone_web_test_roots(repo_root)
        for path in rust_module_graph(
            repo_root, root, configurations[0] | {"__test__"}, allow_opaque_macros=True
        )
    )
    release_graph = page_graph | providers
    return {
        "page_feature_rust_inputs": sorted(page_graph - providers - NON_PAGE_RUST_ALIASES - {PAGE_ONLY_MAIN_PATH}),
        "provider_rust_inputs": {name: sorted(paths) for name, paths in sorted(provider_graphs.items())},
        "test_only_rust_inputs": sorted((test_graph - release_graph) - NON_PAGE_RUST_ALIASES),
        "explicit_non_page_aliases": sorted(NON_PAGE_RUST_ALIASES),
    }


PAGE_MAIN_TEST_MODULE_RE = re.compile(
    r'(?ms)^#\[cfg\(all\(feature\s*=\s*"page",\s*test,\s*not\(target_arch\s*=\s*"wasm32"\)\)\)\]\n'
    r"mod presentation \{\n.*?^\}\n"
)
PAGE_MAIN_MODULE_RE = re.compile(r"^(?P<indent>[ \t]*)(?:pub(?:\([^)]*\))?[ \t]+)?mod[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*;[ \t]*(?:\n|$)")


def _page_main_normal_form(source):
    """Erase only page-gated external module roots from the binary entrypoint."""
    stripped, test_count = PAGE_MAIN_TEST_MODULE_RE.subn("", source)
    if test_count > 1:
        raise ValueError("multiple native test-only presentation roots in main.rs")
    lines = stripped.splitlines(keepends=True)
    output = []
    index = 0
    while index < len(lines):
        start = index
        attributes = []
        while index < len(lines) and lines[index].startswith("#[") and lines[index].rstrip().endswith("]"):
            attributes.append(rust_lex(lines[index]))
            index += 1
        match = PAGE_MAIN_MODULE_RE.match(lines[index]) if index < len(lines) else None
        if match and not match.group("indent") and attributes:
            configurations = page_build_feature_configs(REPO)
            page_enabled, _ = _module_attributes(attributes, configurations[0])
            provider_enabled = any(
                _module_attributes(attributes, feature_set)[0]
                for feature_set in configurations[1:]
            )
            if page_enabled and not provider_enabled:
                index += 1
                continue
        output.extend(lines[start:index])
        if index == start:
            output.append(lines[index])
            index += 1
    return "".join(output)


def page_main_delta_is_page_only(baseline_source, candidate_source):
    """Allow cfg-page module additions and the native-only test stand-in only."""
    return _page_main_normal_form(baseline_source) == _page_main_normal_form(candidate_source)


def rust_module_registration_signature(source):
    """Hash Rust graph, cfg, include, macro, and import tokens across a leaf."""
    tokens = rust_lex(source)
    values = [value for _, value in tokens]
    open_to_close = {"[": "]", "{": "}", "(": ")"}
    safe_expression_macros = {"rsx", "format", "format_args", "vec", "matches"}
    registrations = []

    delimiters = []
    for _, value in tokens:
        if value in open_to_close:
            delimiters.append(open_to_close[value])
        elif value in ("]", "}", ")"):
            if not delimiters or value != delimiters.pop():
                raise ValueError(f"malformed Rust token stream: mismatched delimiter {value!r}")
    if delimiters:
        raise ValueError(f"malformed Rust token stream: unmatched delimiter {delimiters[-1]!r}")

    def group_end(start):
        if start >= len(values) or values[start] not in open_to_close:
            raise ValueError("unbalanced Rust registration token group")
        stack = [open_to_close[values[start]]]
        cursor = start + 1
        while cursor < len(values) and stack:
            value = values[cursor]
            if value in open_to_close:
                stack.append(open_to_close[value])
            elif value in ("]", "}", ")"):
                if not stack or value != stack.pop():
                    raise ValueError("mismatched Rust registration token group")
            cursor += 1
        if stack:
            raise ValueError("unterminated Rust registration token group")
        return cursor

    def statement_end(start):
        stack = []
        cursor = start
        while cursor < len(values):
            value = values[cursor]
            if value in open_to_close:
                stack.append(open_to_close[value])
            elif value in ("]", "}", ")"):
                if not stack or value != stack.pop():
                    raise ValueError("mismatched Rust statement token group")
            elif value == ";" and not stack:
                return cursor + 1
            cursor += 1
        raise ValueError("unterminated Rust registration statement")

    index = 0
    while index < len(values):
        value = values[index]
        if (
            value == "#"
            and index + 1 < len(values)
            and (values[index + 1] == "[" or (
                values[index + 1] == "!" and index + 2 < len(values) and values[index + 2] == "["
            ))
        ):
            attribute_open = index + 2 if values[index + 1] == "!" else index + 1
            end = group_end(attribute_open)
            registrations.append(tokens[index:end])
            index = end
            continue
        if tokens[index][0] == "ident" and value == "mod":
            qualifier = index
            if qualifier and values[qualifier - 1] == "unsafe":
                qualifier -= 1
            # Preserve visibility (`pub` / `pub(crate)`) as part of a module
            # declaration without including neighboring leaf behavior tokens.
            if qualifier and values[qualifier - 1] == ")":
                open_index = qualifier - 2
                nesting = 1
                while open_index >= 0 and nesting:
                    if values[open_index] == ")":
                        nesting += 1
                    elif values[open_index] == "(":
                        nesting -= 1
                    open_index -= 1
                candidate = open_index + 1
                if candidate > 0 and values[candidate - 1] == "pub":
                    qualifier = candidate - 1
            elif qualifier and values[qualifier - 1] == "pub":
                qualifier -= 1
            start = qualifier
            end = index + 1
            while end < len(values) and values[end] not in (";", "{"):
                end += 1
            if end < len(values):
                end += 1
            registrations.append(tokens[start:end])
        if tokens[index][0] == "ident" and value == "use":
            start = index
            qualifier = index
            if qualifier and values[qualifier - 1] == ")":
                open_index = qualifier - 2
                nesting = 1
                while open_index >= 0 and nesting:
                    if values[open_index] == ")":
                        nesting += 1
                    elif values[open_index] == "(":
                        nesting -= 1
                    open_index -= 1
                candidate = open_index + 1
                if candidate > 0 and values[candidate - 1] == "pub":
                    start = candidate - 1
            elif qualifier and values[qualifier - 1] == "pub":
                start = qualifier - 1
            registrations.append(tokens[start:statement_end(index)])
        if (
            tokens[index][0] == "ident"
            and value == "extern"
            and index + 1 < len(values)
            and values[index + 1] == "crate"
        ):
            registrations.append(tokens[index:statement_end(index)])
        if tokens[index][0] == "ident" and value == "macro" and index + 1 < len(values):
            end = index + 1
            while end < len(values) and values[end] not in open_to_close:
                end += 1
            if end < len(values):
                end = group_end(end)
            registrations.append(tokens[index:end])
        if tokens[index][0] == "ident" and value == "macro_rules" and index + 1 < len(values) and values[index + 1] == "!":
            end = index + 2
            while end < len(values) and values[end] not in ("{", "("):
                end += 1
            if end < len(values):
                end = group_end(end)
            registrations.append(tokens[index:end])
        if tokens[index][0] == "ident" and value in {"include", "include_str", "include_bytes"} and index + 1 < len(values) and values[index + 1] == "!":
            end = index + 2
            while end < len(values) and values[end] not in open_to_close:
                end += 1
            if end < len(values):
                end = group_end(end)
            registrations.append(tokens[index:end])
        if (
            tokens[index][0] == "ident"
            and index + 1 < len(values)
            and values[index + 1] == "!"
            and value not in safe_expression_macros
            and value not in {"macro_rules", "include", "include_str", "include_bytes"}
        ):
            end = index + 2
            while end < len(values) and values[end] not in open_to_close:
                end += 1
            if end < len(values):
                end = group_end(end)
            registrations.append(tokens[index:end])
        index += 1

    encoded = json.dumps(registrations, ensure_ascii=True, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def sources():
    # Include relevant untracked source files as well as tracked inputs. This
    # makes adding a module/source fail closed before it is committed. Generated
    # build outputs have explicit exclusions above and their generators remain
    # in the source inventory.
    tracked_and_unignored = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=REPO
    ).decode().split("\0")
    ignored_untracked = subprocess.check_output(
        ["git", "ls-files", "--others", "--ignored", "--exclude-standard", "-z"], cwd=REPO
    ).decode().split("\0")
    # Match the provider's recursive model enumeration, including newly added
    # source models that have not yet been staged in Git.
    vendor = REPO / "ergogen/library/vendor"
    paths = sorted(set(tracked_and_unignored) | set(ignored_untracked) | {
        path.relative_to(REPO).as_posix()
        for path in vendor.glob("*/3d_models/**/*")
        if path.is_file() and path.suffix.lower() in {".step", ".stp", ".stl", ".wrl"}
    })
    return {name: digest(REPO / name) for name in paths if name and
            not name.startswith(GENERATED_SOURCE_PATHS) and
            not {"node_modules", "__pycache__", "target", "dist"}.intersection(Path(name).parts) and
            (name.startswith(("web/", "application/", "core/", "contracts/", "renderer/", "cad/", "kicad/src/", "app/src/", "app/public/", "scripts/", "ergogen/")) or name in ROOT_BUILD_INPUTS) and (REPO / name).is_file()}


def valid_build_id(value):
    return bool(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9-]*", value))


def sha256_file(path):
    try:
        return digest(path)
    except OSError as error:
        raise ValueError(f"required file is missing or unreadable: {path}: {error}") from error


def current_tools():
    tools = [
        ("rustc-version", ["rustc", "--version"]),
        ("cargo-version", ["cargo", "--version"]),
        ("dx-version", ["dx", "--version"]),
        ("wasm-pack-version", ["wasm-pack", "--version"]),
        ("node-version", ["node", "--version"]),
        ("pnpm-version", ["pnpm", "--version"]),
    ]
    observed = {}
    for label, command in tools:
        try:
            observed[label] = subprocess.check_output(command, cwd=REPO, text=True, stderr=subprocess.STDOUT).strip()
        except (OSError, subprocess.CalledProcessError) as error:
            raise ValueError(f"could not capture {label}: {error}") from error
    return observed


def baseline_tools(provenance, baseline):
    expected = {}
    for label in ("rustc-version", "cargo-version", "dx-version", "wasm-pack-version", "node-version", "pnpm-version"):
        matches = [row for row in provenance["commands"] if Path(row.get("log", "")).name == f"{label}.log"]
        if len(matches) != 1 or matches[0].get("exit") != 0:
            raise ValueError(f"baseline has no unique successful {label} receipt")
        log = Path(matches[0]["log"])
        if not log.is_absolute():
            log = baseline / log
        log = log.resolve()
        try:
            log.relative_to(baseline)
        except ValueError as error:
            raise ValueError(f"baseline {label} receipt escapes the build directory") from error
        sha256_file(log)  # also ensures the retained receipt exists
        try:
            expected[label] = log.read_text().strip()
        except OSError as error:
            raise ValueError(f"could not read baseline {label} receipt: {error}") from error
    return expected


def page_check_command():
    return ["cargo", "check", "--manifest-path", "web/Cargo.toml", "--locked", "--target",
            "wasm32-unknown-unknown", "--no-default-features", "--features", "page", "--bin",
            "boardstudio-web"]


def expected_full_commands(baseline, *, include_page_check=True):
    output = baseline
    commands = [
        ("rustc-version", ["rustc", "--version"], REPO, {}),
        ("cargo-version", ["cargo", "--version"], REPO, {}),
        ("dx-version", ["dx", "--version"], REPO, {}),
        ("wasm-pack-version", ["wasm-pack", "--version"], REPO, {}),
        ("node-version", ["node", "--version"], REPO, {}),
        ("pnpm-version", ["pnpm", "--version"], REPO, {}),
    ]
    if include_page_check:
        commands.append(("page-check", page_check_command(), REPO, {}))
    commands.extend([
        ("core", ["wasm-pack", "build", REPO / "core", "--target", "web", "--release", "--locked"], REPO, {}),
        ("core-worker", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "m1_core_worker", "--out-dir", output / "core-worker", "--release", "--locked", "--no-default-features", "--features", "core-worker"], REPO, {}),
        ("cad-worker", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "m1_cad_worker", "--out-dir", output / "cad-worker", "--release", "--locked", "--no-default-features", "--features", "cad-worker"], REPO, {}),
        ("renderer", ["wasm-pack", "build", REPO / "renderer", "--target", "web", "--out-dir", output / "renderer", "--out-name", "boardstudio_renderer_wasm", "--release", "--locked"], REPO, {}),
        ("cad", ["pnpm", "--dir", "cad", "run", "build:wasm"], REPO, {}),
        ("fixtures", ["node", REPO / "scripts/prepare-m1-fixtures.mjs", output / "fixtures"], REPO, {}),
        ("ergogen-catalogue", ["pnpm", "--dir", "ergogen", "run", "prepare:catalog"], REPO, {}),
        ("layout-generators", ["node", REPO / "scripts/web/build-layout-generators.mjs", WEB / "assets"], REPO, {}),
        ("preview-generator", ["node", REPO / "scripts/web/build-preview-generator.mjs", WEB / "assets"], REPO, {}),
        ("ergogen-models", [sys.executable, REPO / "scripts/stage-ergogen-models.py", "--source-root", REPO / "ergogen/library/vendor", "--destination", WEB / "assets/ergogen-models", "--manifest", output / "ergogen-models-catalog.json"], REPO, {}),
    ])
    for mode, prefix in (("root", "/"), ("subpath", "/boardstudio/")):
        commands.append((f"page-{mode}", ["dx", "build", "--web", "--release", "--base-path", prefix, "--no-default-features", "--features", "page", "--cargo-args=--locked"], WEB, {}))
        manifest = output / f"offline-manifest-{mode}.json"
        commands.append((f"offline-worker-{mode}", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "boardstudio_offline_worker", "--out-dir", output / f"offline-{mode}", "--release", "--locked", "--no-default-features", "--features", "service-worker"], REPO, {"BOARDSTUDIO_OFFLINE_MANIFEST": str(manifest)}))
        destination = output / f"site-{mode}"
        if mode == "subpath":
            destination /= "boardstudio"
        commands.append((f"embed-offline-{mode}", ["node", REPO / "scripts/web/embed-worker-wasm.mjs", output / f"offline-{mode}", manifest, destination / "service-worker.js"], REPO, {}))
    return [(label, list(map(str, argv)), str(cwd), environment) for label, argv, cwd, environment in commands]


def checked_baseline(build_id):
    if not valid_build_id(build_id):
        raise ValueError("baseline build id must be alphanumeric with optional hyphens")
    root = BUILD_ROOT.resolve()
    baseline = (BUILD_ROOT / build_id).resolve()
    if baseline.parent != root or not baseline.is_dir():
        raise ValueError("baseline must be an existing direct child of the builds directory")
    provenance_path = baseline / "provenance.json"
    try:
        provenance_path.resolve().relative_to(baseline)
    except ValueError as error:
        raise ValueError("baseline provenance escapes the build directory") from error
    provenance_hash = sha256_file(provenance_path)
    try:
        provenance = json.loads(provenance_path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"baseline provenance is malformed: {error}") from error
    if not isinstance(provenance, dict):
        raise ValueError("baseline provenance must contain a JSON object")
    if provenance.get("reuse_mode") or provenance.get("status") != "complete":
        raise ValueError("reuse-derived or incomplete baselines cannot be reused")
    if not re.fullmatch(r"[0-9a-f]{40}", str(provenance.get("source_commit", ""))):
        raise ValueError("baseline source commit identity is missing or malformed")
    try:
        subprocess.check_output(["git", "cat-file", "-e", f"{provenance['source_commit']}^{{commit}}"], cwd=REPO, stderr=subprocess.PIPE)
    except (OSError, subprocess.CalledProcessError) as error:
        raise ValueError("baseline source commit is unavailable in this repository") from error
    if (not isinstance(provenance.get("sources"), dict) or not provenance["sources"]
            or not all(isinstance(path, str) for path in provenance["sources"])):
        raise ValueError("baseline source manifest is missing")
    if any(not re.fullmatch(r"[0-9a-f]{64}", str(value)) for value in provenance["sources"].values()):
        raise ValueError("baseline source manifest contains an invalid SHA-256")
    if not isinstance(provenance.get("commands"), list) or len(provenance["commands"]) not in (22, 23):
        raise ValueError("baseline is not a complete full build command sequence")
    if not all(isinstance(row, dict) and isinstance(row.get("log"), str) for row in provenance["commands"]):
        raise ValueError("baseline command receipts are malformed")
    command_labels = [Path(row.get("log", "")).name.removesuffix(".log") for row in provenance["commands"]]
    legacy_schema = len(provenance["commands"]) == 22
    required_labels = ["rustc-version", "cargo-version", "dx-version", "wasm-pack-version", "node-version", "pnpm-version"]
    if not legacy_schema:
        required_labels.append("page-check")
    required_labels += [
                       "core", "core-worker", "cad-worker", "renderer", "cad", "fixtures", "ergogen-catalogue",
                       "layout-generators", "preview-generator", "ergogen-models", "page-root", "offline-worker-root",
                       "embed-offline-root", "page-subpath", "offline-worker-subpath", "embed-offline-subpath"]
    if command_labels != required_labels or any(row.get("exit") != 0 for row in provenance["commands"]):
        raise ValueError("baseline command lineage is not the successful full-build sequence")
    if legacy_schema:
        helper_hash = provenance["sources"].get(REUSE_HELPER_PATH)
        compatible_blob = COMPATIBLE_FULL_BUILD_HELPERS.get((provenance["source_commit"], helper_hash))
        if compatible_blob is None:
            raise ValueError("legacy baseline helper is neither unchanged nor the pinned compatible full-build helper")
        try:
            helper_blob = subprocess.check_output(
                ["git", "rev-parse", f"{provenance['source_commit']}:{REUSE_HELPER_PATH}"],
                cwd=REPO, text=True, stderr=subprocess.PIPE,
            ).strip()
        except (OSError, subprocess.CalledProcessError) as error:
            raise ValueError("legacy baseline helper Git identity cannot be verified") from error
        if helper_blob != compatible_blob:
            raise ValueError("legacy baseline helper Git identity differs from its pinned proof")
    expected_commands = expected_full_commands(baseline, include_page_check=not legacy_schema)
    command_log_hashes = {}
    for row, (label, expected_argv, expected_cwd, expected_env) in zip(provenance["commands"], expected_commands):
        log = Path(row.get("log", ""))
        if not log.is_absolute():
            log = baseline / log
        log = log.resolve()
        try:
            log.relative_to(baseline)
        except ValueError as error:
            raise ValueError(f"baseline command log escapes the build directory: {label}") from error
        if not log.is_file():
            raise ValueError(f"baseline command log is missing: {label}")
        command_log_hashes[label] = sha256_file(log)
        argv = row.get("argv")
        if (argv != expected_argv or row.get("cwd") != expected_cwd or row.get("environment", {}) != expected_env):
            raise ValueError(f"baseline full-build command identity is unexpected: {label}")
    for mode, prefix in (("root", "/"), ("subpath", "/boardstudio/")):
        route = provenance.get(mode)
        if not isinstance(route, dict) or route.get("prefix") != prefix or not isinstance(route.get("assets"), dict):
            raise ValueError(f"baseline {mode} route manifest is missing or has the wrong prefix")
        if not isinstance(route.get("site"), str) or not all(
            isinstance(path, str) and isinstance(value, str) for path, value in route["assets"].items()
        ):
            raise ValueError(f"baseline {mode} route manifest is malformed")
        site = Path(route["site"])
        if not site.is_absolute():
            site = baseline / site
        site = site.resolve()
        try:
            site.relative_to(baseline)
        except ValueError as error:
            raise ValueError(f"baseline {mode} site escapes the build directory") from error
        if not site.is_dir():
            raise ValueError(f"baseline {mode} site directory is missing")
        actual = {p.relative_to(site).as_posix() for p in site.rglob("*") if p.is_file()}
        recorded = set(route["assets"])
        if actual != recorded:
            raise ValueError(f"baseline {mode} asset path set differs from provenance")
        required_assets = {
            "assets/m1.css", "assets/core-worker/entry.js", "assets/core-worker/m1_core_worker_bg.wasm",
            "assets/cad-worker/entry.js", "assets/cad-worker/m1_cad_worker_bg.wasm",
            "assets/cad/boardstudio_cadrum_wasm_bg.wasm", "assets/renderer/boardstudio_renderer_wasm_bg.wasm",
            "assets/fixtures/provenance.json", "service-worker.js", "boardstudio_offline_worker.js",
        }
        if not required_assets.issubset(recorded):
            raise ValueError(f"baseline {mode} is missing required provider/staged assets: {sorted(required_assets - recorded)}")
        for relative, expected in route["assets"].items():
            asset = (site / relative).resolve()
            try:
                asset.relative_to(site)
            except ValueError as error:
                raise ValueError(f"baseline {mode} asset escapes its site: {relative}") from error
            if sha256_file(asset) != expected:
                raise ValueError(f"baseline {mode} asset hash differs: {relative}")
    expected_tools = baseline_tools(provenance, baseline)
    observed_tools = current_tools()
    if observed_tools != expected_tools:
        raise ValueError("current tool versions differ from the verified baseline")
    return baseline, provenance, provenance_path, provenance_hash, expected_tools, command_log_hashes


def verified_reuse_helper(provenance, current_helper_hash, *, refresh_fixtures=False):
    """Keep ordinary reuse unchanged; explicit refresh may use the pinned helper."""
    source_commit = provenance["source_commit"]
    baseline_helper_hash = provenance["sources"].get(REUSE_HELPER_PATH)
    if not isinstance(baseline_helper_hash, str):
        raise ValueError("baseline source manifest is missing the guarded reuse helper")
    compatible_blob = COMPATIBLE_FULL_BUILD_HELPERS.get((source_commit, baseline_helper_hash))
    if compatible_blob is not None and refresh_fixtures:
        try:
            blob = subprocess.check_output(
                ["git", "rev-parse", f"{source_commit}:{REUSE_HELPER_PATH}"],
                cwd=REPO,
                text=True,
                stderr=subprocess.PIPE,
            ).strip()
        except (OSError, subprocess.CalledProcessError) as error:
            raise ValueError("compatible full-build helper source cannot be verified") from error
        if blob != compatible_blob:
            raise ValueError("compatible full-build helper Git identity differs")
        compatibility = "pinned-bbd4-full-build-helper"
    elif baseline_helper_hash == current_helper_hash:
        try:
            baseline_bytes = subprocess.check_output(
                ["git", "show", f"{source_commit}:{REUSE_HELPER_PATH}"],
                cwd=REPO,
                stderr=subprocess.PIPE,
            )
        except (OSError, subprocess.CalledProcessError) as error:
            raise ValueError("baseline helper source cannot be verified") from error
        if hashlib.sha256(baseline_bytes).hexdigest() != baseline_helper_hash:
            raise ValueError("baseline helper source does not match its source manifest")
        compatibility = "identical-committed-helper"
    else:
        raise ValueError("baseline helper is neither unchanged nor the pinned compatible full-build helper")
    return {
        "compatibility": compatibility,
        "baseline_source_commit": source_commit,
        "baseline_sha256": baseline_helper_hash,
        "current_sha256": current_helper_hash,
        "baseline_git_blob": compatible_blob,
    }


def validate_reuse(build_id, baseline_id, *, refresh_fixtures=False):
    if not valid_build_id(build_id):
        raise ValueError("build id must be alphanumeric with optional hyphens")
    if not valid_build_id(baseline_id):
        raise ValueError("baseline build id must be alphanumeric with optional hyphens")
    output = BUILD_ROOT.resolve() / build_id
    if output.exists() or output.is_symlink():
        raise ValueError(f"output already exists: {output}")
    baseline, provenance, provenance_path, provenance_hash, tools, command_log_hashes = checked_baseline(baseline_id)
    current = sources()
    old = provenance["sources"]
    if not refresh_fixtures and current.get(FIXTURE_PREPARATION_PATH) != old.get(FIXTURE_PREPARATION_PATH):
        raise ValueError("fixture-preparation changes require explicit --refresh-fixtures-from")
    helper_compatibility = verified_reuse_helper(
        provenance, current.get(REUSE_HELPER_PATH, ""), refresh_fixtures=refresh_fixtures
    )
    added = set(current) - set(old)
    removed = set(old) - set(current)
    if removed:
        raise ValueError(f"source path removals require a fresh full build: {sorted(removed)[:8]}")
    changed = sorted(added | {path for path in current if path in old and current[path] != old[path]})
    ownership = page_feature_ownership(REPO)
    page_rust = set(ownership["page_feature_rust_inputs"])
    test_rust = set(ownership["test_only_rust_inputs"])
    if not page_rust.issubset(current) or not test_rust.issubset(current):
        missing = sorted((page_rust | test_rust) - set(current))
        raise ValueError(f"page module graph is missing source-manifest inputs: {missing[:8]}")
    providers = set().union(*(set(paths) for paths in ownership["provider_rust_inputs"].values()))
    eligible = (PAGE_ONLY_ALLOWLIST | page_rust | test_rust | CORE_TEST_ONLY_PATHS | BUILD_TEST_ONLY_PATHS |
                BUILD_CONTROL_ONLY_PATHS |
                {PAGE_ONLY_MAIN_PATH, REUSE_HELPER_PATH}) - providers - NON_PAGE_RUST_ALIASES
    if refresh_fixtures:
        eligible |= {FIXTURE_PREPARATION_PATH}
        if FIXTURE_PREPARATION_PATH not in changed:
            raise ValueError("fixture refresh requires a changed fixture-preparation source")
    elif FIXTURE_PREPARATION_PATH in changed:
        raise ValueError("fixture-preparation changes require explicit --refresh-fixtures-from")
    disallowed = sorted(set(changed) - eligible)
    if disallowed:
        raise ValueError(f"changed inputs are outside page/test-only ownership: {disallowed}")
    for path in PAGE_ONLY_PROOF_PATHS:
        if path not in old or current.get(path) != old[path]:
            raise ValueError(f"page-only dependency proof input changed or is absent: {path}")
    if not set(PAGE_ONLY_ALLOWLIST).issubset(old):
        raise ValueError("baseline does not contain every audited page-only input")
    syntax_signatures = {}
    for path in changed:
        try:
            head_bytes = subprocess.check_output(["git", "show", f"HEAD:{path}"], cwd=REPO, stderr=subprocess.PIPE)
        except (OSError, subprocess.CalledProcessError) as error:
            raise ValueError(f"changed source is absent from the current committed candidate: {path}") from error
        if hashlib.sha256(head_bytes).hexdigest() != current[path]:
            raise ValueError(f"page-owned Rust or asset input has uncommitted changes: {path}")
        if path in old and path != REUSE_HELPER_PATH:
            try:
                baseline_bytes = subprocess.check_output(
                    ["git", "show", f"{provenance['source_commit']}:{path}"], cwd=REPO, stderr=subprocess.PIPE
                )
            except (OSError, subprocess.CalledProcessError) as error:
                raise ValueError(f"baseline source commit does not contain recorded input: {path}") from error
            if hashlib.sha256(baseline_bytes).hexdigest() != old[path]:
                raise ValueError(f"baseline source manifest does not match its recorded commit: {path}")
            if path.endswith(".rs"):
                # Validate lexical balance on both sides, while ownership is
                # established by the active page/provider module graphs.
                rust_module_registration_signature(baseline_bytes)
                syntax_signatures[path] = rust_module_registration_signature(head_bytes)
        elif path.endswith(".rs"):
            syntax_signatures[path] = rust_module_registration_signature(head_bytes)
    if PAGE_ONLY_MAIN_PATH in changed:
        try:
            baseline_main = subprocess.check_output(
                ["git", "show", f"{provenance['source_commit']}:{PAGE_ONLY_MAIN_PATH}"], cwd=REPO, stderr=subprocess.PIPE
            )
            current_main = subprocess.check_output(
                ["git", "show", f"HEAD:{PAGE_ONLY_MAIN_PATH}"], cwd=REPO, stderr=subprocess.PIPE
            )
        except (OSError, subprocess.CalledProcessError) as error:
            raise ValueError("main.rs page-root ownership cannot be verified") from error
        if hashlib.sha256(baseline_main).hexdigest() != old.get(PAGE_ONLY_MAIN_PATH):
            raise ValueError("baseline main.rs does not match its source manifest")
        if not page_main_delta_is_page_only(baseline_main.decode("utf-8"), current_main.decode("utf-8")):
            raise ValueError("main.rs changed outside cfg-page module roots or the native test stand-in")
    return (output, baseline, provenance, provenance_path, provenance_hash, current,
            changed, tools, command_log_hashes, syntax_signatures, ownership,
            helper_compatibility)


def ignore_rebuilt_assets(directory, names):
    if Path(directory) != WEB / "assets":
        return []
    return [name for name in names if name in {
        "cad", "cad-worker", "core-worker", "renderer", "fixtures", "ergogen-models",
        "layout-generators", "layout-generators.js", "preview-generator",
    }]


def build_reuse(build_id, baseline_id, *, refresh_fixtures=False):
    (output, baseline, base_provenance, baseline_provenance_path,
     baseline_provenance_hash, source_before, changed, tools, command_log_hashes,
     syntax_signatures, ownership, helper_compatibility) = validate_reuse(
         build_id, baseline_id, refresh_fixtures=refresh_fixtures
     )
    # All guards run before this point. Reserve output only after validation.
    output.mkdir(parents=True, exist_ok=False)
    (output / "tmp").mkdir()
    environment = dict(os.environ, TMPDIR=str(output / "tmp"))
    provenance = {
        "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=REPO, text=True).strip(),
        "build_id": build_id,
        "reuse_mode": "fixture-refresh-provider-reuse" if refresh_fixtures else "page-only-provider-reuse",
        "scope": (
            "Fresh fixtures, page, and offline route packages; verified full-build providers reused."
            if refresh_fixtures else
            "Fresh page and offline route packages; verified full-build providers reused."
        ),
        "status": "running",
        "base_build": str(baseline),
        "base_source_commit": base_provenance["source_commit"],
        "base_provenance_sha256": baseline_provenance_hash,
        "base_provenance": str(baseline_provenance_path),
        "inherited_full_build_commands": len(base_provenance["commands"]),
        "inherited_full_build_lineage": [
            {
                "label": Path(row["log"]).stem,
                "argv": row["argv"],
                "cwd": row["cwd"],
                "exit": row["exit"],
                "environment": row.get("environment", {}),
                "log": row["log"],
                "log_sha256": command_log_hashes[Path(row["log"]).stem],
            }
            for row in base_provenance["commands"]
        ],
        "changed_allowlisted_inputs": changed,
        "fixture_refresh": refresh_fixtures,
        "helper_compatibility": helper_compatibility,
        "audited_page_leaf_inputs": {
            name: list(paths) for name, paths in PAGE_ONLY_LEAF_PATHS.items()
        },
        "changed_rust_syntax_signatures": syntax_signatures,
        "page_feature_ownership": ownership,
        "build_test_only_inputs": sorted(BUILD_TEST_ONLY_PATHS),
        "changed_build_test_only_inputs": sorted(set(changed) & BUILD_TEST_ONLY_PATHS),
        "build_control_only_inputs": sorted(BUILD_CONTROL_ONLY_PATHS),
        "changed_build_control_inputs": sorted(set(changed) & BUILD_CONTROL_ONLY_PATHS),
        "dependency_proof": {
            "statement": "Page-only Rust inputs are resolved from the exact page binary and library module roots under the locked wasm32 page feature set, then subtracted against the three provider library roots from the full build command matrix. New/moved module inputs must remain inside that derived page-only graph; deletions require a full build. Separate cfg(test) module files, the single unchanged-Core-target integration test, and the named standalone build-test harnesses are recorded as test-only and are not package command inputs. The exact migration build guard is recorded as a build-control-only input; unrelated scripts still require unchanged provider input or an explicit fixture/helper proof. Cargo manifests, lib root, build script, and main.rs retain their source/feature proofs. The build helper must match the baseline or pass its explicit committed compatibility proof; a fixture-preparation source change is accepted only in fixture-refresh mode. The layout_align_geometry core-worker test alias remains explicitly excluded.",
            "source_hashes": {
                **{path: source_before[path] for path in PAGE_ONLY_PROOF_PATHS},
                PAGE_ONLY_MAIN_PATH: source_before[PAGE_ONLY_MAIN_PATH],
            },
            "reuse_helper": helper_compatibility,
            "build_features": {
                "page": sorted(page_build_feature_configs(REPO)[0]),
                "provider": [sorted(features) for features in page_build_feature_configs(REPO)[1:]],
            },
        },
        "sources": source_before,
        "tool_observations": tools,
        "commands": [],
    }
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")

    def run(name, command, cwd=REPO, extra_env=None):
        print(f"{name}: {' '.join(map(str,command))}", flush=True)
        log = output / f"{name}.log"
        started = datetime.now(timezone.utc).isoformat()
        with log.open("w") as stream:
            result = subprocess.run(list(map(str, command)), cwd=cwd, env=dict(environment, **(extra_env or {})), stdout=stream, stderr=subprocess.STDOUT)
        provenance["commands"].append({"argv": list(map(str, command)), "cwd": str(cwd), "exit": result.returncode, "log": str(log), "environment": extra_env or {}, "started": started, "finished": datetime.now(timezone.utc).isoformat()})
        if result.returncode:
            provenance["status"] = f"failed-{name}"
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        if result.returncode:
            print(log.read_text()[-10000:], file=sys.stderr)
            raise SystemExit(result.returncode)

    run("page-check", page_check_command())
    if refresh_fixtures:
        run("fixtures", ["node", REPO / FIXTURE_PREPARATION_PATH, output / "fixtures"])
    else:
        run("layout-generators", ["node", REPO / "scripts/web/build-layout-generators.mjs", output / "layout-generator-assets"])
        run("preview-generator", ["node", REPO / "scripts/web/build-preview-generator.mjs", output / "preview-generator-assets"])
    for mode, prefix in [("root", "/"), ("subpath", "/boardstudio/")]:
        public = WEB / "target/dx/boardstudio-web/release/web/public"
        # Keep the existing CLI behavior: preserve Dioxus's public output before
        # asking it to emit the fresh page route.
        if public.exists():
            shutil.move(public, output / f"previous-dx-public-{mode}")
        run(f"page-{mode}", ["dx", "build", "--web", "--release", "--base-path", prefix, "--no-default-features", "--features", "page", "--cargo-args=--locked"], WEB)
        destination = output / f"site-{mode}"
        if mode == "subpath":
            destination /= "boardstudio"
        route = base_provenance[mode]
        destination.mkdir(parents=True, exist_ok=False)
        assets = destination / "assets"
        assets.mkdir()
        # Stage only the exact inherited providers from the validated baseline.
        # The candidate site never starts as a copy of the prior site, so stale
        # or unrelated baseline files cannot leak into this route.
        inherited_prefixes = FIXTURE_REFRESH_INHERITED_PREFIXES if refresh_fixtures else REUSED_PROVIDER_PREFIXES
        inherited_providers = {
            path: file_hash for path, file_hash in route["assets"].items()
            if path.startswith(inherited_prefixes)
        }
        for relative in sorted(inherited_providers):
            source = Path(route["site"]) / relative
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        shutil.copytree(public, destination, dirs_exist_ok=True)
        shutil.copytree(WEB / "assets", assets, dirs_exist_ok=True, ignore=ignore_rebuilt_assets)
        if refresh_fixtures:
            shutil.copytree(output / "fixtures", assets / "fixtures", dirs_exist_ok=True)
        else:
            shutil.copytree(output / "layout-generator-assets", assets, dirs_exist_ok=True)
            shutil.copytree(output / "preview-generator-assets", assets, dirs_exist_ok=True)
        manifest = output / f"offline-manifest-{mode}.json"
        required = sorted({str(path.relative_to(destination)) for path in destination.rglob("*") if path.is_file()} | {"service-worker.js", "boardstudio_offline_worker.js"})
        manifest.write_text(json.dumps({"version": f"{build_id}-{mode}", "assets": required}, indent=2)+"\n")
        run(f"offline-worker-{mode}", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "boardstudio_offline_worker", "--out-dir", output / f"offline-{mode}", "--release", "--locked", "--no-default-features", "--features", "service-worker"], extra_env={"BOARDSTUDIO_OFFLINE_MANIFEST": str(manifest)})
        run(f"embed-offline-{mode}", ["node", REPO / "scripts/web/embed-worker-wasm.mjs", output / f"offline-{mode}", manifest, destination / "service-worker.js"])
        provenance[mode] = {"prefix": prefix, "site": str(destination), "assets": {
            str(path.relative_to(destination)): digest(path) for path in sorted(destination.rglob("*")) if path.is_file()}}
        candidate_providers = {
            path: file_hash for path, file_hash in provenance[mode]["assets"].items()
            if path.startswith(inherited_prefixes)
        }
        if candidate_providers != inherited_providers:
            provenance["status"] = "failed-reused-provider-drift"
            provenance["reused_provider_error"] = mode
            (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
            raise SystemExit(f"Reused provider path set or bytes changed during staging: {mode}")
        for path, expected in inherited_providers.items():
            if provenance[mode]["assets"].get(path) != expected:
                provenance["status"] = "failed-reused-provider-drift"
                provenance["reused_provider_error"] = f"{mode}:{path}"
                (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
                raise SystemExit(f"Reused provider asset changed during staging: {mode}:{path}")
        provenance[mode]["reused_provider_assets"] = inherited_providers
        if refresh_fixtures:
            fixture_assets = {
                path: file_hash for path, file_hash in provenance[mode]["assets"].items()
                if path.startswith("assets/fixtures/")
            }
            fresh_fixture_assets = {
                f"assets/fixtures/{path.relative_to(output / 'fixtures').as_posix()}": sha256_file(path)
                for path in sorted((output / "fixtures").rglob("*")) if path.is_file()
            }
            if fixture_assets != fresh_fixture_assets:
                provenance["status"] = "failed-fixture-refresh-drift"
                provenance["fixture_refresh_error"] = mode
                (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
                raise SystemExit(f"Refreshed fixture path set or bytes changed during staging: {mode}")
            provenance[mode]["refreshed_fixture_assets"] = fixture_assets
        provenance["commands"][-1]["site_manifest_sha256"] = digest(manifest)
        provenance["commands"][-1]["site_asset_count"] = len(provenance[mode]["assets"])
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
    source_after = sources()
    if source_after != source_before:
        provenance["status"] = "failed-source-drift"
        provenance["source_drift"] = sorted(
            path for path in set(source_before) | set(source_after)
            if source_before.get(path) != source_after.get(path)
        )
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        raise SystemExit("Source changed during build: candidate has mixed provenance and must be rebuilt.")
    if sha256_file(baseline_provenance_path) != baseline_provenance_hash:
        provenance["status"] = "failed-baseline-drift"
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        raise SystemExit("Baseline provenance changed during build: candidate must be rebuilt.")
    # Full baseline validation includes every route path, asset hash, provenance
    # identity and tool receipt. Source identity is compared to the pre-build
    # snapshot above because the exact allowlisted page delta is intentional.
    try:
        post_baseline = checked_baseline(baseline_id)
        if post_baseline[3] != baseline_provenance_hash or post_baseline[5] != command_log_hashes:
            raise ValueError("baseline provenance changed during build")
    except ValueError as error:
        provenance["status"] = "failed-baseline-drift"
        provenance["baseline_validation_error"] = str(error)
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        raise SystemExit(f"Baseline changed during build: {error}") from error
    provenance["status"] = "complete"
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
    print(output, flush=True)


def build_full(build_id):
    output = BUILD_ROOT / build_id
    output.mkdir(parents=True, exist_ok=False)
    (output / "tmp").mkdir()
    environment = dict(os.environ, TMPDIR=str(output / "tmp"))
    provenance = {"source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=REPO, text=True).strip(),
                  "build_id": build_id,
                  "sources": sources(), "commands": [], "scope": "Development candidate; acceptance is recorded separately.",
                  "status": "running"}

    def run(name, command, cwd=REPO, extra_env=None):
        print(f"{name}: {' '.join(map(str,command))}", flush=True)
        log = output / f"{name}.log"
        started = datetime.now(timezone.utc).isoformat()
        with log.open("w") as stream:
            result = subprocess.run(list(map(str, command)), cwd=cwd, env=dict(environment, **(extra_env or {})), stdout=stream, stderr=subprocess.STDOUT)
        provenance["commands"].append({"argv": list(map(str, command)), "cwd": str(cwd), "exit": result.returncode, "log": str(log), "environment": extra_env or {}, "started": started, "finished": datetime.now(timezone.utc).isoformat()})
        if result.returncode:
            provenance["status"] = f"failed-{name}"
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        if result.returncode:
            print(log.read_text()[-10000:], file=sys.stderr)
            raise SystemExit(result.returncode)

    for name, command in [("rustc-version", ["rustc", "--version"]), ("cargo-version", ["cargo", "--version"]), ("dx-version", ["dx", "--version"]), ("wasm-pack-version", ["wasm-pack", "--version"]), ("node-version", ["node", "--version"]), ("pnpm-version", ["pnpm", "--version"]),
                          ("page-check", page_check_command()),
                          ("core", ["wasm-pack", "build", REPO / "core", "--target", "web", "--release", "--locked"]),
                          ("core-worker", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "m1_core_worker", "--out-dir", output / "core-worker", "--release", "--locked", "--no-default-features", "--features", "core-worker"]),
                          ("cad-worker", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "m1_cad_worker", "--out-dir", output / "cad-worker", "--release", "--locked", "--no-default-features", "--features", "cad-worker"]),
                          ("renderer", ["wasm-pack", "build", REPO / "renderer", "--target", "web", "--out-dir", output / "renderer", "--out-name", "boardstudio_renderer_wasm", "--release", "--locked"]),
                          ("cad", ["pnpm", "--dir", "cad", "run", "build:wasm"]),
                          ("fixtures", ["node", REPO / "scripts/prepare-m1-fixtures.mjs", output / "fixtures"]),
                          ("ergogen-catalogue", ["pnpm", "--dir", "ergogen", "run", "prepare:catalog"]),
                          ("layout-generators", ["node", REPO / "scripts/web/build-layout-generators.mjs", WEB / "assets"]),
                          ("preview-generator", ["node", REPO / "scripts/web/build-preview-generator.mjs", WEB / "assets"]),
                          ("ergogen-models", [sys.executable, REPO / "scripts/stage-ergogen-models.py", "--source-root", REPO / "ergogen/library/vendor", "--destination", WEB / "assets/ergogen-models", "--manifest", output / "ergogen-models-catalog.json"])]:
        run(name, command)
    for mode, prefix in (("root", "/"), ("subpath", "/boardstudio/")):
        public = WEB / "target/dx/boardstudio-web/release/web/public"
        if public.exists():
            shutil.move(public, output / f"previous-dx-public-{mode}")
        run(f"page-{mode}", ["dx", "build", "--web", "--release", "--base-path", prefix, "--no-default-features", "--features", "page", "--cargo-args=--locked"], WEB)
        destination = output / f"site-{mode}"
        if mode == "subpath":
            destination /= "boardstudio"
        shutil.copytree(public, destination)
        assets = destination / "assets"
        shutil.copytree(WEB / "assets", assets, dirs_exist_ok=True)
        shutil.copytree(REPO / "cad/wasm/pkg", assets / "cad", dirs_exist_ok=True)
        for name in ("core-worker", "cad-worker", "renderer", "fixtures"):
            shutil.copytree(output / name, assets / name, dirs_exist_ok=True)
        (assets / "core-worker/entry.js").write_text('import init, { start_core_worker } from "./m1_core_worker.js";\nawait init();\nstart_core_worker();\n')
        (assets / "cad-worker/entry.js").write_text('import init, { start_cad_worker } from "./m1_cad_worker.js";\nawait init();\nstart_cad_worker(new URL("../cad/boardstudio_cadrum_wasm.js", import.meta.url).href);\n')
        manifest = output / f"offline-manifest-{mode}.json"
        required = sorted({str(path.relative_to(destination)) for path in destination.rglob("*") if path.is_file()} | {"service-worker.js", "boardstudio_offline_worker.js"})
        manifest.write_text(json.dumps({"version": f"{build_id}-{mode}", "assets": required}, indent=2)+"\n")
        run(f"offline-worker-{mode}", ["wasm-pack", "build", WEB, "--target", "web", "--out-name", "boardstudio_offline_worker", "--out-dir", output / f"offline-{mode}", "--release", "--locked", "--no-default-features", "--features", "service-worker"], extra_env={"BOARDSTUDIO_OFFLINE_MANIFEST": str(manifest)})
        run(f"embed-offline-{mode}", ["node", REPO / "scripts/web/embed-worker-wasm.mjs", output / f"offline-{mode}", manifest, destination / "service-worker.js"])
        provenance[mode] = {"prefix": prefix, "site": str(destination), "assets": {str(path.relative_to(destination)): digest(path) for path in sorted(destination.rglob("*")) if path.is_file()}}
    source_after = sources()
    if source_after != provenance["sources"]:
        provenance["status"] = "failed-source-drift"
        provenance["source_drift"] = sorted(
            path for path in set(provenance["sources"]) | set(source_after)
            if provenance["sources"].get(path) != source_after.get(path)
        )
        (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
        raise SystemExit("Source changed during build: candidate has mixed provenance and must be rebuilt.")
    provenance["status"] = "complete"
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2)+"\n")
    print(output, flush=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description="Build a full M1 candidate or a guarded page-only reuse candidate.")
    parser.add_argument("build_id", help="unique alphanumeric/hyphen build identifier")
    reuse = parser.add_mutually_exclusive_group()
    reuse.add_argument("--reuse-providers-from", metavar="FULL_BUILD_ID", help="reuse verified providers from a complete full build")
    reuse.add_argument("--refresh-fixtures-from", metavar="FULL_BUILD_ID", help="refresh demo fixtures and page/offline routes from a compatible complete full build")
    args = parser.parse_args(argv)
    if not valid_build_id(args.build_id):
        parser.error("build id must be alphanumeric with optional hyphens")
    if args.reuse_providers_from is not None and not valid_build_id(args.reuse_providers_from):
        parser.error("baseline build id must be alphanumeric with optional hyphens")
    if args.refresh_fixtures_from is not None and not valid_build_id(args.refresh_fixtures_from):
        parser.error("baseline build id must be alphanumeric with optional hyphens")
    guard_spec = importlib.util.spec_from_file_location(
        "migration_deliver", REPO / "scripts/migration-deliver.py"
    )
    if guard_spec is None or guard_spec.loader is None:
        raise RuntimeError("migration delivery build guard cannot be loaded")
    guard = importlib.util.module_from_spec(guard_spec)
    guard_spec.loader.exec_module(guard)
    build_freeze = guard.build_freeze
    with build_freeze(REPO):
        if args.reuse_providers_from is not None:
            try:
                build_reuse(args.build_id, args.reuse_providers_from)
            except ValueError as error:
                parser.error(str(error))
            return
        if args.refresh_fixtures_from is not None:
            try:
                build_reuse(args.build_id, args.refresh_fixtures_from, refresh_fixtures=True)
            except ValueError as error:
                parser.error(str(error))
            return
        build_full(args.build_id)


if __name__ == "__main__":
    main()
