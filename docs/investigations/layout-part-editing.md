# Layout part editing: ownership and queued coordinates

Investigated 2026-10-05 at `3cdeb2ac2`. Status: open finding; the queued overwrite
is reproduced, while live browser reproduction, repair and interface design
remain open. Application behavior was not changed.

Layout part editing spreads draft behavior, selection ownership and command
construction across the Inspector and shared presentation module. The most useful
deepening opportunity is ownership of an edit through acceptance. A synthetic
Session/Core reproduction demonstrates a queued Y edit restoring the old X after
an X edit commits. Live browser reproduction remains outstanding.

## Current ownership

| Responsibility | Source |
| --- | --- |
| Drafts, two-decimal display, Enter/blur suppression, Escape | `LayoutComponentInspector` in [the Inspector form](../../web/src/presentation/inspector/layout_component_inspector.rs) |
| Projection, selection lifetime, owner admission, command construction | `layout_component_inspector_projection`, `layout_component_inspector_owner_is_current`, and `dispatch_layout_component_inspector_action` in [presentation.rs](../../web/src/presentation.rs) |
| Browser effects and operation delivery | [Runtime](../../web/crates/runtime/src/runtime.rs) |
| Ordered edits and accepted state | [Session](../../application/src/session.rs) |
| Document edits and Undo/Redo | [CoreEngine](../../core/src/lib.rs) |

The form already has a compact interface: a projection, the selected Inspector
tab and an action handler. Shared composition constructs its lifetime, projection
and admission context separately. The mounted test host repeats that wiring.
Moving these functions between files would improve navigation, but depth requires
reducing the lifecycle knowledge callers must coordinate.

## Confirmed command-sequence reproduction

`SetPosition` starts from the accepted part position, replaces one axis, and
constructs absolute `MoveParts` positions for the selection. If X and Y actions
are admitted before either completes, both commands can use the same starting
snapshot. Session refreshes a queued non-strict edit's `base_revision` before
execution; it does not recompute the operation's position payload.

The [companion reproducer](layout-part-editing-repro.rs) submits Inspector-shaped
commands through the real public Session/Core path, including successful
persistence completions and Undo:

| Step | X | Y |
| --- | ---: | ---: |
| Initial accepted position | 66.675 | -47.625 |
| First command: set X to 60 | 60 | -47.625 |
| Queued command: set Y to -40, built from the initial position | 66.675 | -40 |
| Final accepted position, revision 2 | **66.675** | **-40** |
| First Undo, revision 3 | 60 | -47.625 |
| Second Undo, revision 4 | 66.675 | -47.625 |

The first edit committed before the second overwrote X. This confirms the queued
payload behavior. The reproducer directly supplies the command sequence: it does
not mount the Inspector, test its admission checks, or establish how often live
browser timing permits this sequence.

Run from the repository root. Cargo files and build output stay in a fresh OS
temporary directory; the example imports the current checkout's public types.

```sh
repo_root=$(git rev-parse --show-toplevel)
repro_dir=$(mktemp -d "${TMPDIR:-/tmp}/layout-part-editing.XXXXXX")
python3 - "$repo_root" "$repro_dir" <<'PY'
import json
from pathlib import Path
import sys

root, scratch = map(Path, sys.argv[1:])
q = lambda path: json.dumps(str(path))
(scratch / "Cargo.toml").write_text(f'''
[package]
name = "layout-part-editing-repro"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "layout-part-editing-repro"
path = {q(root / "docs/investigations/layout-part-editing-repro.rs")}

[dependencies]
boardstudio-application = {{ path = {q(root / "application")} }}
boardstudio_core = {{ path = {q(root / "core")} }}
''')
PY
cargo run --manifest-path "$repro_dir/Cargo.toml" --target-dir "$repro_dir/target"
```

This is diagnostic evidence rather than a permanent regression test: its output
records accepted coordinates without asserting that the overwrite must persist.

## What the existing tests establish

The focused browser run passed **17 tests, zero failures** on 2026-10-05:

```sh
python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs
```

[The suite](../../web/src/presentation/layout_component_inspector_tests.rs) mounts
the form and calls production projection and dispatch logic. It covers stale
selection callbacks, selection away-and-back, Inspector removal, unrelated
accepted revisions, draft and tab retention, Enter/blur event counts, untouched
precision, Escape, locked/driven positions, and group translation payloads.

Its Runtime uses `set_layout_component_inspector_test_state`. In that mode,
`Runtime::submit` records events and returns before calling Session. Consequently,
`mounted_rapid_xy_enter_and_focus_change_submit_one_edit_per_axis` proves that two
commands are emitted, but does not verify the final accepted X/Y or Undo. Fixture
updates simulate unrelated acceptance by replacing the test model.

## Constraints on a future deepening

- Keep selection lifetime distinct from accepted snapshot identity. An unrelated
  revision refreshes action admission while preserving unaffected drafts; leaving
  and returning to a selection invalidates retained actions.
- Preserve raw geometry when an untouched two-decimal field blurs. Enter remains
  explicit commit intent, and its following blur must not duplicate that commit.
- Preserve first-selected-part anchoring and one translation for group edits,
  including locked and relationship-driven eligibility.
- Keep the accepted document, persistence and history authoritative in
  Session/Core. Presentation drafts must not become a second document store.
- Retain browser tests for focus and event ordering. Add accepted-result coverage
  through the feature seam so operation capture alone cannot stand in for a
  completed edit.
- Keep matrix-transform policy separate during this investigation: its form
  already tracks explicit pending outcomes and baseline conflicts.

The useful module would concentrate scoped edit intent and settlement, giving
locality to changes and leverage to callers and tests. Its implementation can
retain separate projection, form and execution adapters. A generic numeric-edit
framework or another adapter that only captures events would not resolve the
acceptance gap.

## Remaining work

1. Reproduce rapid X/Y input through the mounted Inspector and real edit path,
   controlling completion timing and checking accepted coordinates and Undo.
2. Decide where queued field intent is resolved against accepted state, and how
   pending edits, rejection and retry should appear to the user.
3. Select a bounded module interface only after that behavior is settled; retain
   existing precision and owner-admission regressions during implementation.

Track the unresolved behavior in the [backlog](../backlog.md). This investigation
does not establish a new architectural decision or reopen either outline ADR.
