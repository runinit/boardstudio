# Repository cleanup inventory

This inventory records documentation that was reviewed during the 2026-09
cleanup. It separates current operating guidance from historical planning so
future cleanup can remove stale plans without losing decisions, measurements,
or provenance.

## Retain as current guidance

- [`architecture.md`](architecture.md) is the maintained ownership, contract,
  compatibility, and validation reference.
- [`mechanical-assemblies.md`](mechanical-assemblies.md) documents the current
  mechanical profile and export boundaries.
- [`assembly-preview.md`](assembly-preview.md) documents the current preview
  behavior and its limits.
- [`memory-workflow.md`](memory-workflow.md) documents the project memory
  process.
- [`performance-baseline.md`](performance-baseline.md) is the active benchmark
  record and threshold source.
- Coverage and migration evidence from the former `test-audit.md` is retained
  in the current architecture and validation records; the superseded report is
  removed below.
- The design documents remain references for interaction intent. They may
  contain planned follow-up, but their implemented sections should not be
  treated as a current API or format contract.

Vendor, upstream, and licensing material under `ergogen/` is outside this
inventory and must remain available with its provenance and original notices.

## Historical dispositions

These files describe completed migration work or alternatives that are no
longer active implementation plans. Before removal, preserve any unique
decision, measurement, or source reference in the current architecture,
performance, or this inventory.

| File | Evidence | Disposition |
| --- | --- | --- |
| `rust-migration-implementation.md` | Historical validation report; current Rust ownership is in `architecture.md`. | Removed; current validation entrypoints are retained in `architecture.md`. |
| `rust-migration-roadmap.md` | Matrix, archive, and CAD migrations are complete; UI migration was exploratory only. | Removed; no frontend cutover is part of the current architecture. |
| `rust-migration-research.md` | Unapproved recommendations mixed completed work with speculative migrations. | Removed; selected boundaries are recorded in `architecture.md`. |
| `kicad-rust-migration.md` | Completed serializer migration report with obsolete migration framing. | Removed; current KiCad ownership is in `architecture.md`. |
| `rust-matrix-archives.md` | Completed archive/projection migration report with obsolete importer references. | Removed; current project compatibility is in `architecture.md`. |
| `test-audit.md` | Historical migration and coverage report superseded by current validation docs. | Removed; maintained checks are in `architecture.md` and `README.md`. |
| `cadrum-assessment.md` | Assessment of an alternative CAD adapter; current implementation and limits are in `architecture.md` and `cad-kernel-options.md`. | Retained as research because it records upstream constraints and rejected integration paths. |
| `opencascade-rs-assessment.md` | Alternative wrapper assessment with proposed experiments rather than a selected implementation. | Retained as research because it records why the current adapter boundary remains explicit. |
| `pure-rust-cad-assessment.md` | Alternative kernel survey; no selected backend or implementation dependency. | Retained as research because it documents evaluated replacement options and evidence limits. |
| `frontend-framework-comparison.md` | Explicitly a planning comparison; no frontend migration is authorized. | Retained as research because it records the no-cutover decision and its tradeoffs. |

The following are not removal candidates: `cad-kernel-options.md` remains the
consolidated CAD decision record; the mirrored-layout and workbench design
documents preserve interaction rationale; and vendor/reference documents must
retain their upstream attribution and licenses.

## Removed local remnants

`app/src/context/AGENTS.md` was a stale, ignored scoped instruction file: it
references removed `ConfigContext`, JSCAD worker, and legacy migration paths.
The ignored `engine/` tree contained generated output, dependency links, and
ignored guidance. Both directories were removed after verifying that no tracked
file or current runtime depended on them. Working Impeccable context symlinks
remain in place.

## Automated checks and their limits

`pnpm check:repo` runs scanner fixtures before checking the working tree. It is
included in `pnpm check`, which CI runs on pull requests and pushes to `main`
and `dev`.

The scanner parses authored TypeScript/JavaScript imports and exports. Production
roots include the app, worker entrypoints, benchmark pages, and workspace package
facades. Test imports count as export consumers but do not make test-only modules
production-reachable. Generated contracts and vendor output retain their existing
contract/catalogue integrity checks. Contract and CAD facades preserve their
runtime API boundaries; their explicit exceptions live beside the entrypoints in
the checker.

Namespace and literal dynamic imports conservatively consume the target module's
exports. Computed imports, computed package exports, and runtime reflection need
manual review; this is not proof of whole-program dead-code absence. Runtime
`dependencies` are checked per package; build tools and `devDependencies` are not
classified as unused from source imports alone. No dependency was removed without
an identified unused declaration.

Local Markdown destinations are checked in root and package READMEs, product and
design documents, and `docs/`. The check verifies files/directories, not remote
URLs, heading anchors, or code examples. The stale CasePreview link now points to
the current AssemblyScene.

The initial pass narrowed unused helper/type exports and benchmark functions
exposed only through `window`. Their implementations remain in use. The runtime
audit found no confirmed removable backend compatibility paths: supported KiCad
syntax, reversible-jumper validation, and geometry fallbacks remain active.

## Validation record — 2026-09-26

The cleanup snapshot based on `bc406fe9` passed a frozen install and regeneration
in a separate worktree, native/package and type checks, 199 app tests, 153 browser
scenarios, Pages, development-server checks, and performance gates.

The subsequent frontend work passed 227 app tests and covered all 156 functional
browser scenarios. Existing new-project fixtures now explicitly skip the optional
guide; compact drawer selectors target the header rather than the new Objects
navigation button. After those fixture corrections, all 22 scenarios in the three
affected browser files passed. Native/package checks, production builds, contract
and boundary checks, nine development-server tests, Pages, and the full performance
gate also passed. Desktop and narrow visual checks covered both themes, focus,
horizontal overflow, and compact guide targets. Changes remained uncommitted on
`dev` during validation; no project schema or renderer protocol changed.
