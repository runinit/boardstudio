# 04: Validate generator parameters at the Core boundary

Status: implemented (awaiting verification)
Type: build
Blocked by: 03
Map: [map.md](../map.md) · Decision: [ADR-0009](../../../adr/0009-generator-parameters-checked-in-core.md)

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

## Outcome

Status: implemented (awaiting verification). Not marked resolved: the checks below ran on
`71b23c165`, the commit before this docs commit; the code did not change after it. Decision:
[ADR-0009](../../../adr/0009-generator-parameters-checked-in-core.md).

### What changed

- `footprints/src/params.rs`, `registry.rs`: `ParameterProblem` (`WrongType` with the declared kind
  and the found value, `Undeclared`), and `Registry::check_parameters`, which applies
  `ParamKind::accepts` to one saved map in name order. A null value is absent, as at render.
- `core/src/generators.rs`: one check over every saved map: a definition's own parameters at the
  definition, a part's own at the part, a member's at the member. Each value is judged once. Wrappers:
  `document_error` (Open and ReplaceDocument refuse wrong types, except non-empty list text),
  `introduced_error` (an edit refuses what it introduces or changes), `definition_error` (strict, for
  the web test), and `parameter_findings` (warnings and errors through `validate`).
- `core/src/lib.rs`: Open calls `document_error`; the commit path of `edit` calls `introduced_error`
  after all mutations and before the document is replaced, and ReplaceDocument is exempt there;
  `preview_edit` checks the same way inside its closure (its backup restores on error); the
  ReplaceDocument arm of `apply` calls `document_error`. `core/src/validate.rs` adds the findings.
- Divergence from the ticket: the ticket said "merged parameters". A part's merged map would repeat a
  definition's unknown name on every part and hide a wrong definition value that a part overrides. Each
  map is therefore checked where it is stored, which judges the same values and gives one finding per
  location.
- Divergence in policy: non-empty list text is a warning on Open and ReplaceDocument, not a refusal,
  because it can occur in saved data (see inventory). It is refused on any edit that writes it.

### Inventory (before the Open policy was coded)

- Demo projects: regenerated from current code with `core/examples/prepare_demo_projects.rs`, and from
  the build's `web/assets/fixtures`: 20 `.boardstudio` at format 4 (`reviung41.boardstudio` is the
  unmigrated v1 copy, format absent, migrated on unpack). Checked by a Python pre-inventory
  (declared names from `footprints/src/generators/**`, walking definitions, parts and members) and by
  the native acceptance test, which now asserts no `generator-parameter:` finding on each opened demo.
- Also checked: `content/archives/reviung41-original.boardstudio` (v1, unpacked and opened through
  `archive::request`), `core/tests/fixtures/reviung41-outline-original.json`,
  `web/crates/layout/tests/fixtures/sofle-v2.json`, the boundary transcript documents (zero findings in
  the first pass), and `catalogue/parts/imported-parts.json` (nine KiCad-imported parts with no generator,
  so nothing to check).
- Result: 0 undeclared names, 0 wrong types, 0 non-empty list text across all of them. The pre-inventory
  holds 6,106 saved generator parameter values (793 on definitions, 5,313 on parts). The pre-inventory
  first reported 1,171 undeclared names, all caused by a regex that skipped uppercase net names (`GND`,
  `P0`); after the fix none remain.
- Non-empty list text: no shipped data holds it, and no array-kind value is stored in any bundled
  fixture at all. Version 4 keeps `"[1,2,3]"` as text, so older saves might hold it; the policy warns
  rather than refuses. This is not proven either way.

### Tests (red first)

- `footprints/tests/parameter_check.rs` (6 tests): written against a stub that returns no problems.
  Red: 5 failed on their assertions; the defaults test passed trivially. Green after the implementation (6 passed).
- `core/tests/generator_parameters.rs` (13 tests, in the `integration` binary): written against no-op
  checks. Red, counted before the preview-reply fix described below: 11 failed on their assertions (Open
  and edits accepted, findings missing). 2 passed under the stub, and they are guards: a legacy list text
  does not block an unrelated edit, and the shipped archive opens with no finding. After the fix the
  move-preview guard also passes under the stub. Green after the implementation: the 13 pass (the name filter also runs
  3 closure tests, so the filtered run reported 16 passed).
- A test-harness detail found in red: a preview phase returns `CoreReply::Preview`, not `Scene`. The
  move-preview test expected `Scene` and was corrected before the implementation was written.
- `core/tests/demo_projects.rs`: an added assertion that no opened demo carries a `generator-parameter:`
  finding (it passed).
- `web/crates/parts/src/parts/generator_settings.rs`: a `core_check_tests` browser module (3 tests):
  submitted numbers and JSON lists pass `definition_error`; text that does not parse is refused in the
  form; a negative control shows the check catches a value written past the form. These passed on the
  existing coercion, so they guard it rather than drive a change (not red first). The parse is `wasm32`-only
  (`pub mod parts` is `cfg(target_arch = "wasm32")`), so there is no native test of `apply_input`.

### Commands and observed results (at `71b23c165`)

- `cargo test --manifest-path core/Cargo.toml --lib --test integration` (nested): 274 lib passed,
  494 integration passed, 0 failed (boundary parity included, native).
- `cargo test --manifest-path footprints/Cargo.toml` (nested): all binaries passed, 0 failed.
- `cargo fmt --all --check`: clean (after formatting my three new or touched test files).
- `cargo clippy --workspace --locked --all-targets --no-deps -- -D warnings`: exit 0.
- `TMPDIR=$HOME/.cache/tce-verify-tmp python3 scripts/check.py repo lint typecheck build` (nested
  worktree, foreground): `check: all steps passed: repo, lint, typecheck, build`, exit 0.
- `python3 scripts/check.py browser` (nested, foreground, sandbox disabled for wasm-pack and Chrome):
  `check: all steps passed: browser`, exit 0; `run-wasm-tests: executed 70, failed 0`. The
  `wasm-pack test ... web/crates/parts` step includes the new `core_check_tests`; run alone with the
  filter `core_check_tests` it reported 3 passed.
- `python3 scripts/check.py test` in `/tmp/claude-1000/boardstudio-tce-verify` detached at `71b23c165`
  (foreground): `check: all steps passed: test`, exit 0, 149.5 s.
- Background runs of `check.py repo lint typecheck build browser` at the same commit: two ended with
  `check: all steps passed: repo, lint, typecheck, build, browser` (totals 522 s and 378 s; logs
  `scratchpad/nested-check-killed.log` and `nested-check-killed2.log`). They overlapped on the shared build
  directory. A third overlapping run exited with status 1; I had deleted its log, so it is not evidence
  either way. The foreground runs above are the record for those steps.
- `check.py repo` on the ticket and map edits: passed (0.1 s, 147 Markdown files).

### GitNexus

- The index is stale (indexed at `4b372b2`; the CLI says so). Pre-edit impact (upstream):
  `validate` CRITICAL (29 impacted), `apply` CRITICAL (32), `CoreEngine::edit` CRITICAL (47),
  `handle` CRITICAL (69), `preview_edit` CRITICAL (26), `render` LOW (not edited). Each edit is
  additive: a new check call, with the existing behaviour kept when nothing is refused or warned.
  The risk was reported before each edit and covered by the native and browser suites above.
- `detect-changes --scope all`: 8 files, 19 symbols, 9 affected processes, risk high. The stale index
  maps by line range, so it lists symbols I did not change (inserted code shifted their lines). It also
  reports the diff touched files whose hunks overlap no indexed symbol ("not a clean tree").

### Not verified or not done

- `check.py release` was not run (not allowed). `check.py quick` was not run (the ticket named the steps).
  `security` and `deny` were not run (no dependency change).
- Preview cost was not measured. The check walks the saved parameter maps twice per edit (before and
  after), and once per preview edit, so the cost grows with saved generator parameters.
- `preview_edit`'s own refusal path has no test: its operations (MoveParts, matrix operations) cannot
  write generator parameters. Preview refusal is tested through the generic edit path (AddPart in
  preview phase). The `preview_edit` check is a guard and does not change behaviour today.
- A part copied with a legacy undeclared name (for example through a mirror) is refused, because the
  copy writes the name. The shipped data has no such name. Recorded in ADR-0009.
- A saved value that is non-numeric text for a number (for example `"two"`) now refuses Open. The
  migration keeps it, and no shipped project holds one.
- The web browser tests were not red first (see Tests).
- The `unused import Severity` warning in `web/crates/case/src/case_generation_admission.rs` appears in the
  wasm check; it was already in the base commit and is not from this change.
- Commit trailer: `Co-Authored-By: Claude Sonnet 5.5`, as the lane instruction specified; the session
  environment reports Haiku 5.5.
