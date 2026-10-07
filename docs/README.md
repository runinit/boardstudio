# Documentation

Start with [development and checks](../README.md), [architecture](architecture.md),
and the [domain vocabulary](../CONTEXT.md). The [backlog](backlog.md) tracks open
problems and completion conditions.

## Current references

| Area | Read when |
| --- | --- |
| [Architecture](architecture.md) | Locating ownership across Rust and browser providers |
| [Component onboarding](hardware/component-onboarding.md) | Changing footprints, hardware definitions or associated models |
| [Assembly previews](features/assembly-preview.md) | Working on Parts samples, models or routed PCB references |
| [Mechanical assemblies](features/mechanical-assemblies.md) | Working on the generated Case stack, manufacturing or exports |
| [Keycaps and keymap](features/keycaps-and-keymap.md) | Working on keycap geometry, bindings or firmware export |
| [THQWGD001 qualification](hardware/thqwgd001.md) | Checking nominal encoder assemblies and unresolved hardware facts |
| [VIK qualification](hardware/vik.md) | Inspecting retained VIK data and qualification limits; new catalogue choices remain deferred |

## Investigations

[Layout part editing](investigations/layout-part-editing.md) records the ownership
review, passing presentation tests, and a reproduced queued-coordinate overwrite.
It includes a runnable Session/Core reproducer. The live browser timing and the
eventual repair remain open; no replacement interface has been selected.

Investigations distinguish observations, reproductions and proposals. Record the
inspected revision, reproduction steps, verification limits and remaining work so
the evidence can be checked before implementation.

[Rust conventions review](investigations/rust-conventions-review.md) weighs the
installed Rust skills against this codebase, with measured costs and a ranked,
unimplemented change list.

[TypeScript and Node removal](investigations/typescript-node-removal.md) inventories
the remaining provider, content, build and test dependencies, with a phased removal
plan and completion criteria. It is an assessment, not a completed migration.

## Design and decisions

- [Fixed edited outlines](adr/0001-fixed-edited-outlines.md) and
  [linked outline refinements](adr/0002-linked-outline-refinements.md) are accepted
  domain decisions. Their acceptance does not establish implementation completion.
- Edits: [queued edits resolve at execution](adr/0005-resolve-queued-edits-at-execution.md)
  (commits are intents; the direct event carries previews only) and
  [whole-document replacement is for import and recovery](adr/0006-replace-document-for-import-and-recovery.md),
  the direction of the [typed Core edits](plans/typed-core-edits/map.md) effort.
- [Mirrored layout pairs](design/mirrored-layout-pair.md) describes the interaction
  and ownership contract.
- [Gasket case redesign](design/gasket-case-redesign.md) preserves the agreed
  mechanical design and unresolved hardware evidence.
- [ZMK reference](design/zmk-keymap-reference.md) records the dated upstream
  behavior used for local editing and export.

## Research and historical evidence

These notes retain their original dates and measured revisions. Benchmark results,
old code paths and integration status describe those snapshots; use the current
architecture and source for present ownership.

| Topic | Evidence |
| --- | --- |
| CAD selection | [Kernel comparison](research/cad/cad-kernel-options.md), [Cadrum integration](research/cad/cadrum-assessment.md), [opencascade-rs](research/cad/opencascade-rs-assessment.md), [pure Rust alternatives](research/cad/pure-rust-cad-assessment.md) |
| Generation performance | [Measurements](research/performance/generation-performance.md), [experiment plan](research/performance/generation-optimization-experiments.md), [gasket comparisons](research/performance/gasket-comparison-results.md), [continued OCCT screening](research/performance/occt-optimization-round2.md) |
| Mechanical hardware | [Gasket preset research](hardware/gasket-hardware-presets-research.md), including supplier evidence and retained images |

## Archived workflows

The [VIK app review checklist](archive/vik-module-human-review.md) describes the
retired React workflow and test commands. Its review scenarios remain historical
reference material, not instructions for the current Dioxus app.

Migration records and other retired documentation remain in Git at `323967ff`.
For example, `git show 323967ff:docs/development-worktrees.md` retrieves the
historical worktree record. Keep ordinary development instructions in the root
README and accepted ownership in the architecture reference.
