# 183: Align generator editor null values with declared defaults

Status: resolved
Type: build
Blocked by: None
Related: [Generator parameter research](../../../research/2026-10-19-generator-parameter-coherence.md)
Decision: User approved the narrowed scope and existing editor/render/export test seams on 2026-10-19.

## Problem Statement

A project can contain a generator parameter saved as null, which means the
generator should use its declared default. Core accepts this and rendering uses
that default. The Parts editor instead treats null as supplied: a number can
appear blank and block candidate preview, or a boolean can appear unchecked even
when its default is true. Users should not have to repair a valid absent value
before editing their footprint generator settings.

## Solution

Make the Parts generator editor show and preview the same effective defaults as
footprint generation. An absent or null parameter uses the declared default; a
net without a default remains unassigned. Preserve typed form validation, Core
checks and export behavior rather than introducing another policy module.

## User Stories

1. As a board designer, I want a saved-null numeric parameter to show its declared default, so that a valid project does not appear to contain an unfinished field.
2. As a board designer, I want an omitted numeric parameter to behave like saved null, so that storage representation does not change what I see.
3. As a board designer, I want a saved-null boolean to show its declared default, so that I do not mistake a default-enabled feature for a disabled feature.
4. As a board designer, I want saved-null text and list parameters to show their defaults, so that the form represents the footprint actually produced.
5. As a board designer, I want a net without a default to remain unassigned, so that opening settings does not invent electrical connections.
6. As a board designer, I want initial preview to use the values shown in the form, so that I can trust it before editing.
7. As a board designer, I want opening settings and preparing preview to leave the accepted document unchanged, so that inspection creates no edit or Undo entry.
8. As a board designer, I want explicit non-null settings to take precedence, so that the editor preserves my chosen configuration.
9. As a board designer, I want numeric and list draft text converted to declared types, so that valid edits pass Core checks.
10. As a board designer, I want invalid numeric or list drafts to show feedback and prevent Apply, so that malformed settings cannot enter the accepted document.
11. As a board designer, I want empty text distinguished from absence, so that text/net settings retain their meaning and blank numeric drafts remain invalid.
12. As a board designer, I want one setting change to preserve unrelated accepted metadata, so that applying a draft does not undo other work.
13. As a board designer, I want older saved names retained under the warning policy, so that correcting display does not silently discard data.
14. As a board designer, I want part overrides and null-default behavior preserved in rendering and KiCad output, so that an editor fix does not change exports.
15. As a maintainer, I want repeated effective-value selections to follow one local rule, so that a later default fix does not need inconsistent edits.

## Implementation Decisions

- Limit production changes to effective-value interpretation within the Parts generator settings module. Reuse declaration-derived schema, candidate preparation and normalization interfaces.
- Replace inconsistent selection in field projection, initial preview seeding and candidate preparation with the smallest local consolidation. Depth, locality and leverage must come from replacing that repetition, not adding a forwarding module, cross-crate seam, adapter, dependency or schema framework.
- An explicit non-null draft wins over the saved value. An explicit null draft selects the declaration default, not an older saved non-null value. Without a draft, saved null and missing values select the declaration default. A no-default net stays absent/no-net.
- Preserve null versus empty text. Keep finite-number parsing, boolean validation, string/net handling and JSON-array parsing in presentation; do not add text coercion to rendering or Core saved-value acceptance.
- Keep declarations authoritative for names, kinds and defaults. Do not infer kinds from defaults or change the supported kinds.
- Preserve Core ownership of checking each saved definition, placed-part and assembly-member map, including introduced-value refusal and legacy warnings, under ADR-0009.
- Preserve definition-then-part render merging. Part null masks a definition's non-null value and selects the declaration default; it does not inherit the definition value.
- Keep initial preview transient and retain changed-parameter tracking, owner lifetime checks and rebasing onto the accepted document. Displaying a default is not an edit and must not rewrite accepted state.
- Preserve unknown saved names, existing failure feedback and readiness gating.
- Leave electrical terminal reconciliation, model-path rewriting, version checks, stable net allocation and KiCad output assembly in their existing modules.

## Testing Decisions

- **Approved primary seam:** the mounted Parts generator settings interface, using the existing controlled candidate-provider hook plus at least one production candidate preparation/normalization/drawing assertion. Verify fields, preview status, candidate behavior and unchanged accepted state, not helper structure.
- **Approved supplementary existing seam:** Core generator rendering and KiCad export for default/override parity. The definition editor cannot observe placed-part null masking or exported output. No new adapter or test-only public interface is needed.
- Prior art: `mounted_generator_initial_schema_default_seeds_unapplied_preview`, `submitted_numbers_and_lists_pass_the_core_check`, and `text_that_does_not_parse_is_refused_before_it_reaches_core`.
- Start with a failing observable saved-null regression. Cover number, boolean, string and list defaults, including a true boolean default and a net without a default so false/blank fallbacks cannot pass accidentally.
- Compare missing and saved-null behavior; explicit non-null values still win. Exercise explicit null through the existing candidate interface without adding a user control.
- Prove mount/initial preview do not change the accepted document or revision. Retain existing Apply/rebase and failure tests rather than rebuilding edit-settlement coverage.
- Verify blank, malformed and nonfinite numeric drafts, malformed JSON and non-array JSON remain refused and cannot enable Apply. Preserve empty text for kinds that already permit it.
- Use existing generator export tests for non-null part overrides and part null selecting the declared default instead of a non-null definition value. Assert parsed geometry/net properties, not private maps, incidental IDs or broad text snapshots.
- Retain Core parameter tests for name/type checks, per-owner policy, legacy warnings and unrelated edits; do not duplicate their entire matrix in editor tests.
- Use controlled completion and existing DOM waits, not arbitrary sleeps. Run `python3 scripts/check.py browser --only parts` for focused mounted coverage, `python3 scripts/check.py quick` for changed-path native/lint/compile coverage and `python3 scripts/check.py` before closure. Follow the [verification workflow](../../../../README.md#checks), run builds sequentially against the shared target and keep long output/receipts in `.evidence/`. A browser subset is not full acceptance.

## Out of Scope

- New parameter-policy module, public schema redesign or parser framework.
- Changes to declarations, supported kinds, persisted format, migration or legacy policy.
- Changing render precedence, unknown-name handling, net allocation or terminal reconciliation.
- New reset controls, configurable policy or automatic saved-map rewriting.
- Removing stale object/anchor branches solely as cleanup.
- Generator body ports, form escaping, preview geometry, model handling or edit-settlement refactoring.

## Further Notes

The [research](../../../research/2026-10-19-generator-parameter-coherence.md)
records current-source evidence at HEAD `e326d17778524bd1704c76eb71e2c5bb7b773f10`
with substantial existing working-tree edits. Preserve them, particularly export
tests. The broad drift claim was narrowed: validation and export already share
declaration acceptance and rendering. The null-default mismatch is source-based,
not yet reproduced in a browser; a failing regression is the first gate.

The user approved this narrower scope and both test seams on 2026-10-19. There is
no blocking dependency on [182](182-reuse-parsed-kicad-board-source.md) or
[184](184-use-direct-forms-for-plated-mounting-hole.md). This ticket owns editor
effective-value interpretation and narrowly necessary parity coverage. Coordinate
overlapping export tests without changing sibling-owned production behavior.
No implementation or behavioral verification has occurred.
