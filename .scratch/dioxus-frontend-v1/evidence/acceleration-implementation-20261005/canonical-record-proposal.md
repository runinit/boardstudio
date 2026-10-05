# Functional-frontier canonical record proposal

This is a proposal for the record owner; it does not change `tasks.json` or the run record. Preserve prior qualification history and evidence. New or re-scoped journeys below start `pending`; none of the fixture identities authorizes reuse of an earlier verdict.

## Complete maintained-input upper bound

`scripts/build-m1.py:is_build_source_path`, `ROOT_BUILD_INPUTS`, and `sources()` define the maintained source inventory used by candidate provenance. I imported the maintained build helper and enumerated its live `sources()` result, then checked every path against the predicate and the root/exact-file footprint below. The inventory had **1,409 paths, 1,409 covered, 0 uncovered**. Counts by source root were: web 234; application 7; core 106; contracts 248; renderer 54; cad 71; kicad/src 2; app/src 389; app/public 9; scripts 44; ergogen 239. All six present `ROOT_BUILD_INPUTS` files were covered exactly. The two `.cargo/config*` entries are absent from this checkout; `optional_source_paths` pins their absence so their addition invalidates reuse.

```json
{
  "source_roots": [
    "web", "application", "core", "contracts", "renderer", "cad",
    "kicad/src", "app/src", "app/public", "scripts", "ergogen"
  ],
  "source_paths": [
    ".node-version", ".npmrc", "package.json", "pnpm-lock.yaml",
    "pnpm-workspace.yaml", "rust-toolchain.toml"
  ],
  "optional_source_paths": [".cargo/config", ".cargo/config.toml"],
  "source_paths_complete": true
}
```

The roots intentionally include web CSS, JavaScript, HTML/bootstrap, build scripts, worker entry points, application/Core/contracts, renderer and CAD implementation/build inputs, vendored model providers, and the migration scripts. These broad roots over-approximate each journey's specific dependencies; source-root comparison therefore invalidates reuse on any changed or newly added source under them. The explicit paths cover the present root build/toolchain inputs. `optional_source_paths` pins currently absent `ROOT_BUILD_INPUTS` paths: the helper validates them as safe repository-relative paths, requires them to remain absent, compares the list exactly with the previous verdict, and invalidates reuse if one is added even as an untracked file. The existing qualification mechanism also checks that the roots and present files exist and are repository-relative before reuse. This audited upper bound is complete for the current maintained input policy; keep every proposed journey pending until freshly qualified and update its footprint before qualification if the source inventory policy changes.

## Pending journey definitions

Apply the complete-input upper bound above to each definition below. Keep each `state` pending; set `source_paths_complete` true to assert the audited broad inventory, not prior functional qualification or a passed journey. Preserve any old result in its existing history entry without editing its identity or verdict.

### `saved-recipe-layout`

```json
{
  "id": "saved-recipe-layout",
  "scope": "Apply a saved recipe to a real Layout, verify key positions and board origin, re-edit a parameter, reject invalid input, and save/reopen the result.",
  "state": "pending",
  "source_roots": ["web", "application", "core", "contracts", "renderer", "cad", "kicad/src", "app/src", "app/public", "scripts", "ergogen"],
  "source_paths": [".node-version", ".npmrc", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", "rust-toolchain.toml"],
  "optional_source_paths": [".cargo/config", ".cargo/config.toml"],
  "source_paths_complete": true,
  "fixtures": [
    {"path": ".scratch/dioxus-frontend-v1/evidence/pcb-host-themes-20261004/input.boardstudio", "sha256": "7a52b098ff15b03c7a2a5c0f65a3b89ae512731ae695b9b958dae27816b4dbbe"},
    {"path": ".scratch/dioxus-frontend-v1/evidence/pcb-host-themes-20261004/input.json", "sha256": "af47e81045349d2e8a039cd4c62570cc6bcc519ba7bd8624fe0d648d5be1fa51"}
  ]
}
```

The pinned board archive is the supplied future start fixture (two boards, 70 parts each, revision 9, no modules). It is not the prior asymmetric GroupedSofle proof.

### `layout-keymap-keycaps`

```json
{
  "id": "layout-keymap-keycaps",
  "scope": "Starting from the pinned two-board Layout fixture, exercise Layout to Keymap and Keycaps handoff, layer editing, keycap configuration/navigation, and save/reopen behavior.",
  "state": "pending",
  "source_roots": ["web", "application", "core", "contracts", "renderer", "cad", "kicad/src", "app/src", "app/public", "scripts", "ergogen"],
  "source_paths": [".node-version", ".npmrc", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", "rust-toolchain.toml"],
  "optional_source_paths": [".cargo/config", ".cargo/config.toml"],
  "source_paths_complete": true,
  "fixtures": [
    {"path": ".scratch/dioxus-frontend-v1/evidence/pcb-host-themes-20261004/input.boardstudio", "sha256": "7a52b098ff15b03c7a2a5c0f65a3b89ae512731ae695b9b958dae27816b4dbbe"},
    {"path": ".scratch/dioxus-frontend-v1/evidence/pcb-host-themes-20261004/input.json", "sha256": "af47e81045349d2e8a039cd4c62570cc6bcc519ba7bd8624fe0d648d5be1fa51"}
  ]
}
```

### `pcb-model-viewer`

```json
{
  "id": "pcb-model-viewer",
  "scope": "On the pinned generic-module model fixture, qualify PCB model pick, pose adjustment, save, and reopen only.",
  "state": "pending",
  "source_roots": ["web", "application", "core", "contracts", "renderer", "cad", "kicad/src", "app/src", "app/public", "scripts", "ergogen"],
  "source_paths": [".node-version", ".npmrc", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", "rust-toolchain.toml"],
  "optional_source_paths": [".cargo/config", ".cargo/config.toml"],
  "source_paths_complete": true,
  "fixtures": [
    {"path": ".scratch/dioxus-shared-viewer/evidence/public-case-first/generic-module-model-asset-fixture.boardstudio", "sha256": "debf7891f80174c27d6c25ed8d7ae61eabf76d5e1f7fedd6075ee58bd0d9bf6d"}
  ]
}
```

This fixture supports the narrow model-pick/pose/save start state. It does not support claims about circuits, interfaces, constituents, unavailable-asset feedback, or broad Case projection; keep those claims outside this journey unless separately evidenced.

### `case-local-step-scope`

```json
{
  "id": "case-local-step-scope",
  "scope": "From the pinned initial Sofle archive, add a public Plate and configure its mechanical settings, then qualify local STEP export of the authored Case.",
  "state": "pending",
  "source_roots": ["web", "application", "core", "contracts", "renderer", "cad", "kicad/src", "app/src", "app/public", "scripts", "ergogen"],
  "source_paths": [".node-version", ".npmrc", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml", "rust-toolchain.toml"],
  "optional_source_paths": [".cargo/config", ".cargo/config.toml"],
  "source_paths_complete": true,
  "fixtures": [
    {"path": ".scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-layered-public/fixture/initial-sofle.boardstudio", "sha256": "0e1e06beabfce1c5a2d6bfc472c85ed281435ff2382174b32f9a2bd3d6d58899"}
  ]
}
```

Do not qualify STEP export from the fixture alone: the explicit public Add Plate and Configure Mechanical setup must precede export.

## F3.5-C04 desktop-only active action

Retain prior receipts and current implementation history. Replace the active action/finish wording with the desktop scope below; do not count compact/mobile evidence as passed or include it in the active functional frontier.

```json
{
  "next_action": "Complete paired desktop-only public keyboard scenarios for remaining Board and Outline Back focus restoration and the outstanding nested Inspector routes, using valid targets where a route requires one. Mobile and compact-drawer qualification is deferred by user priority and is not established by current receipts.",
  "finish_condition": "Paired desktop public keyboard scenarios prove Inspector tabs, nested routes, Escape/back, and focus restoration across Layout, Board, and Outline contexts. Mobile and compact-drawer scenarios remain deferred and must not be reported as passed."
}
```

## Guardrails for applying the proposal

- Keep every new journey pending until an actual paired journey is freshly qualified.
- Preserve existing qualification history and do not convert a changed fixture identity into a reused pass.
- Keep visual parity and release joins independent of functional readiness.
- Keep mobile/compact entirely deferred in the active report; do not imply they passed.
- Reuse remains eligible only with a future per-journey complete footprint audit and exact unchanged fixture identities.
