# 04: Validate generator parameters at the Core boundary

Status: ready-for-agent
Type: build
Blocked by: 03
Map: [map.md](../map.md)

## Request

Core never type-checks generator parameters on Open or edit (`core/src/validate.rs` has
no generator rule); a wrong type surfaces only at render. The web Parts editor coerces at
its own boundary (`web/crates/parts/src/parts/generator_settings.rs:305-318`). Render
ignores undeclared names (`footprints/tests/framework.rs:255`), so a typo or a stale name
silently uses the default.

## What to do

- Add one shared check (in `footprints` or `core::generators`) built on
  `ParamKind::accepts`: given a source, the merged parameters and their location, return a
  typed problem per wrong type and per undeclared name.
- Edits (`apply` path, `core/src/lib.rs:~522-535`, and `preview_edit`): wrong types and
  unknown names are errors with no mutation, the same pattern as `layout::validate`.
- Open and ReplaceDocument: a wrong type is an error (after 03 only hand-edited data can
  have one, except non-empty list text for an array parameter, which 03 leaves as text; check
  whether real data can hold it and warn instead of refusing if so). An unknown name becomes a located warning finding through
  `validate::validate`, not a refusal. First inventory bundled fixtures and
  `content/archives/reviung41-original.boardstudio` for unknown names and record the result.
- Keep the web editor's form coercion (it parses user text); add a test that what it
  submits passes the Core check.

## Acceptance

- [ ] A string `hole_size` in an edit is refused with the generator and parameter named.
- [ ] An unknown name in an edit is refused; on Open it is a warning finding.
- [ ] Every bundled fixture opens with no new finding (or each one is recorded).
- [ ] A short ADR (0009) records what Core now accepts.

## Verification

Core and application tests, wasm typecheck, `quick`; GitNexus `impact` on the edit-commit
path.

## Risks / ADR

Third-party or older documents may carry unknown names; the warning-only Open policy
covers that. ADR warranted: it changes what Core accepts.
