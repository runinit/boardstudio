# 05: Generator parameter lint test and conventions

Status: resolved
Type: build
Blocked by: 04
Map: [map.md](../map.md)

## Request

Parameter vocabulary is inconsistent (`hole_drill`/`hole_size`, `drill`/`outline`,
`via_drill`/`via_size`, Case `hole_diameter`/`boss_diameter`) and nothing documents how to
declare one. Renaming and new integer/enum kinds are out of scope (see the map).

## What to do

- Add a `footprints/tests/` lint over every registered generator: unique names, snake_case,
  each default accepted by its kind, `side` declared with default `F`, and no new numeric
  parameter declared as `string` unless it is on an allow-list (the allow-list documents
  existing names, one comment per entry).
- Add a Core test: every Core writer of generator parameters (closure projection,
  `electrical_*`, `mechanical/inputs.rs` mirror `side`) produces values the 04 check
  accepts.
- Docs: a "Generator parameters" section in `docs/hardware/component-onboarding.md`
  (declare explicit kinds, millimetre numbers, no numeric strings, a migration for any
  type change, existing vocabulary listed as legacy). In `GLOSSARY.md`, extend
  **Generator parameters**: values are typed by the declaration; Core rejects wrong types
  and undeclared names on edit.

## Acceptance

- [x] The lint fails on a deliberately mistyped default (shown in the Outcome).
- [x] The docs pass `scripts/check-doc-links.py`.

## Verification

`quick`, `repo`. No ADR. Risk: the allow-list can grow quietly.

## Outcome

Status: resolved. Implemented by Haiku in `e51dfa794` (the lint), `6cc81e4ba` (the Core writer test and a
helper refactor) and `0c0de4714` (docs). Verified and corrected by the integrating Sonnet session in
`7bd579ebc` (the lint's name rule, its tests, the docs) and `2ab64f22b` (two more writer tests); see
[Verification](#verification-integrating-sonnet-session-2026-10-09). The sections below are the
implementer's record; where the verification changed something, the text says so.

### What changed

- `footprints/tests/parameter_lint.rs` (new): `raw_problems(spec)` and `lint(spec)` over one
  `GeneratorSpec`. Rules: unique names; snake_case names; each default accepted by its kind (a net may
  have no default); `side` declared as text with default `F`; a text parameter with a numeric-looking
  default or a dimension name (`size`, `drill`, `width`, `offset`, ...) is a numeric-string problem.
  The implementer's version held two exception lists, `NOT_SNAKE_CASE` (138 `(generator, parameter)`
  pairs) and `NUMERIC_STRING`. The verification replaced the first by a rule on the declaration (see
  Verification, finding 1); only `NUMERIC_STRING` (empty) remains. Tests: every registered generator
  passes; the negative tests use synthetic specs; every exception still matches a real declaration.
- `core/tests/generator_parameters.rs`: `every_value_a_core_writer_produces_passes_the_parameter_check`.
  For each instance of each included split keyboard and the reused PCB: the saved document; the
  instance's effective read (a flipped half's mirror writes `side`); for each instance that owns its
  board, the captured document of the KiCad export plan and every job's definition and part. Uses
  `generators::parameter_problems`, `definition_error` and `bundled().check_parameters`.
- `core/tests/included_keyboards.rs`: `export_board`'s request is factored into `export_request`, and
  `projects()` is `pub(crate)`. Behaviour is unchanged (10 tests pass).
- `docs/hardware/component-onboarding.md`: "Generator parameters" section (explicit kinds, millimetre
  numbers, no numeric strings, `side`, snake_case and its exceptions, migrations per ADR-0004, Core's
  check per ADR-0009, links to ADR-0003) and the legacy vocabulary table (corrected by the verification).
- `GLOSSARY.md`: **Generator parameters** extended with the typing rule and the Open and edit behaviour.

### The allow-lists (the audit)

The audit ran the lint over the real registry (36 generators) before any exception was written. It
found no problem other than snake_case: no duplicate, no default rejected by its kind, no `side`
problem, and no numeric-looking text parameter.

- Not snake_case: 138 names, all pad or pin names. 86 are `net_default` and 2 are `net` (such as
  `GND`, `BAT_P`, `P21`, `P2` on the LED); 50 are `string` MCU pin labels (`P21_label`, `RAW_label`).
  The saved key is the pin or pad name and renaming is out of scope (generator-parameters map), so
  snake_case cannot hold on the current names. The implementer listed all 138; the verification
  replaced the list by a rule on the declaration, because the spec exposes a reliable discriminator
  (`ParamKind::Net`, and a text parameter whose stem is a declared net).
- `NUMERIC_STRING`: empty. No bundled text parameter has a numeric-looking default or a dimension
  name. Pinned at 0.
- `side`: every one of the 36 generators declares `side` as text with default `F`, so the rule has no
  exceptions. The golden fixtures record the old JavaScript defaults for six generators (`B`: diode,
  SK6812 LED, Choc, Gateron, MX, trackpoint mount; the implementer wrote five) and for the keepout
  zone (`F&B`), and four fixtures have no `side`; `check_schema` tolerates the seven recorded
  deviations, and the Rust port declares `F`.

### Proof that the lint bites

- Synthetic declaration `hole_size` as `Number` with default `Str("2.2")`, plus `hole_drill` as text
  `"3"`: `lint` returns `DefaultRejected { parameter: "hole_size", kind: Number }` and
  `NumericString("hole_drill")` (the negative test `the_lint_reports_a_mistyped_default_on_a_synthetic_declaration`).
- Mutation, default check disabled: 2 negative tests fail (`the_lint_reports_a_mistyped_default...`,
  `the_lint_reports_a_number_with_no_default`). Restored; green.
- Mutation, numeric-text check disabled: `the_lint_reports_a_mistyped_default...` fails. Restored; green.
- Core writer test, mutation in `core/src/mechanical/inputs.rs`: the mirror's `side` written as a
  boolean instead of `"F"`/`"B"`. The test fails at `reused-pcb flipped: the effective read` with
  `WrongType` findings on `side`. The mutation was reverted; `git diff -- core/src` is empty.

### Commands and observed results

- `cargo test --manifest-path footprints/Cargo.toml --test parameter_lint`: 8 passed, 0 failed (after
  formatting).
- `cargo clippy --manifest-path footprints/Cargo.toml --all-targets --no-deps -- -D warnings`: no issues.
- `python3 scripts/check.py repo` (in the worktree, with the docs edits): repo passed, doc-link check
  passed over 147 Markdown files.
- `python3 scripts/check.py lint` (in the worktree): all steps passed (rustfmt and clippy workspace).
- `core/tests/integration` filter `generator_parameters`: 24 passed, 0 failed (69.8 s). `included_keyboards`: 10 passed, 0 failed (71.3 s).
- `python3 scripts/check.py test` in `/tmp/claude-1000/boardstudio-tce-verify` detached at `6cc81e4ba`:
  `check: all steps passed: test`, exit 0, 96.1 s. Summed test results: 1316 passed, 0 failed; the
  integration binary: 502 passed, 0 failed.
- The first background run of that test step produced only its header and no process; it was not a
  test result. The step was run again and that second run is the record above.

### GitNexus

- The index is stale. `impact` on `export_board` and `projects` did not resolve the `included_keyboards.rs`
  symbols (`not found` by uid; the name resolves to `generator_export.rs`), so risk is UNKNOWN. I edited
  these two helpers without running impact first, which breaks the rule; I ran it afterwards.
  Text search shows their callers are all in `core/tests/included_keyboards.rs`, and the refactor is
  covered by its 10 passing tests.
- `detect-changes --scope all`: risk low, 4 files, 5 symbols, 0 processes, but every listed symbol is a
  documentation section. The Rust test files (the new lint, the writer test, the helper refactor) are
  not mapped by the stale index, so that result is incomplete. No production symbol changed
  (`git diff -- core/src footprints/src web` is empty).

### Not verified or not done

- `check.py quick`, `typecheck`, `build`, `browser`, `deny` and `release` were not run. No web or wasm code
  changed. `release` was not run by rule. (The verification ran `typecheck` and added one native test
  in the web catalogue crate; see below.)
- The physical setup's write of `reversible` was not reached by the Core test; the verification added a
  native test in its own crate (finding 4).
- The lint cannot see values written by web code or saved by other tools; only declared names and
  defaults.
- The writer test takes about 70 s in a debug build, and the integration binary takes 82 s.
- Whether saved documents outside this repository hold numeric text or undeclared names is unknown.
  The lint only covers the bundled declarations.

### Verification (integrating Sonnet session, 2026-10-09)

Verified against `0c0de4714`; corrected in `7bd579ebc` and `2ab64f22b`. The implementer's report was not
trusted; each point was read in the code or run.

What held (re-measured with a throwaway probe test, deleted): the registry holds 36 generators and the
lint iterates `bundled().specs()`, the registry's own list (36 `GeneratorSpec` statics in
`footprints/src/generators`); all 36 declare `side` as text with default `F`; 138 names are not
snake_case, of which 88 are `net` parameters in capitals (`[A-Z][A-Z0-9_]*`) and 50 are text parameters
`<NET>_label` whose stem is one of those nets; none is anything else. The implementer's audit counts were
right (86 `net_default` plus 2 `net`).

Findings and what was done:

1. Mutation checks on the implementer's lint found survivors. Disabling the numeric-default half of the
   numeric-text rule, or the dimension-name half, left all 8 tests green (the one synthetic case,
   `hole_drill` with default `"3"`, tripped both halves, so each masked the other), and so did dropping
   the `kind == String` test from the `side` rule. Fixed with isolated synthetic cases
   (`numeric_text_is_found_by_its_default_and_by_its_name_separately`, `SIDE_NET`).
2. The 138-entry `NOT_SNAKE_CASE` list is replaced by a rule on the declaration. A name passes if it is
   snake_case, or a `net` parameter in capitals (a pad or pin name), or a text parameter `<NET>_label`
   whose stem is such a declared net. The counts the bundled generators use (88 pad names, 50 pin labels)
   are pinned (`PAD_NAMES`, `PIN_LABELS`), so a new pad-named parameter is a reviewed change, while
   a stale per-name entry can no longer exist. Rejected cases are tested: camelCase, capitals on a number
   or text, mixed-case nets, a label with no net, a label of the wrong kind, a label of a mixed-case net.
   `NUMERIC_STRING` stays as a list (empty, pinned at 0) with the reason now part of each entry
   (`(generator, parameter, reason)`); `invalid_exceptions` reports an empty reason, an unregistered
   generator and a stale entry, and tests prove each (the implementer's version could not fail on an
   empty reason, which was only a source comment).
3. The negative tests call the same `lint`/`lint_with` as the registry test (not a copy); with the
   registry deliberately broken (`ceoloide/mounting_hole_npth` `hole_size` renamed `holeSize`) the
   real-registry test fails with `BadName("holeSize")`; reverted. Three of the lint's rules (unique names,
   default of the declared kind, `side`) duplicate `Registry::new`'s own validation, which stops the
   program at start, so the real-registry test can never fail on them; the lint's value there is
   reporting every problem at once. A test now shows `Registry::new` refuses the same synthetic
   declarations. The rules the lint alone holds are the name rule and numeric text.
4. Writer coverage. The implementer's Core test reaches the closure projection (`clearance.rs`, the
   definitions in the effective read), the mirror's `side` (`inputs.rs`) and the KiCad planning
   (`planning.rs terminal_parameters`), but not `electrical::materialize`, which writes terminal net
   names into the saved document when a board's wiring is applied: the recipes do not call it. Added
   `applying_a_boards_wiring_writes_only_values_the_parameter_check_accepts`: for every board of every
   bundled project that names a controller it resolves and applies the wiring through the engine, requires
   that terminal nets were written, and checks the result. The physical setup's `reversible` write is in
   the web catalogue crate, but its `propose` is pure and its unit tests run natively, so the earlier "no
   native test is possible" did not hold: `reversible_writes_a_boolean_that_the_core_parameter_check_accepts`
   proposes the change on a `ceoloide/switch_mx` part and checks the proposed document. Remaining
   writers not run by any test: `electrical.rs` `resolve_jumpers`, which writes `side` only into a
   clone it analyses and never saves.
5. Mutations of the writers, each reverted (non-nested checkout): `electrical::materialize` writing a
   boolean, the closure hole size written as text, and the mirror's `side` written as a boolean, and
   `planning.rs` writing a number for a terminal each fail the Core writer tests. The first two fail while
   the projects are built (the recipes go through the engine, whose own boundary refuses the value), so the
   tests there are a second line; the other two fail in the test's own assertions, which is the case the
   test exists for. The `reversible` write written as text fails the new web catalogue test with a
   `WrongType` on `reversible`.
6. The `included_keyboards.rs` refactor changes no assertion: the diff is the `pub(crate)` on `projects`
   and `export_board`'s request construction moved into `export_request`, unchanged. All 10 tests pass.
7. Docs checked against the code. Corrected: the legacy table omitted the switches and the router for
   `via_size`/`via_drill`, and gave `drill` only for the infused-kim hole and trackpoint mount although the
   plated holes, `infused-kim/mounting_hole` and the trackpoint mount all declare `drill` and `outline`; the
   `side` bullet cited ADR-0008, which says nothing about `side` (link removed); the names bullet described
   the removed list. GLOSSARY.md said an opened project reports "such saved values", but Open refuses a value
   of any other wrong type; it now says an undeclared name or text for a number opens with a warning. The
   vocabulary lists, the ADR-0003/0004/0009 links, "kinds are number, boolean, string, net, array" and the
   rename and integer/enum exclusions match the code and the map. GLOSSARY.md carries no implementation
   detail.
8. Environment trap, not a defect: this checkout and `/tmp/claude-1000/boardstudio-tce-verify` share one
   `target` directory. A mutation made in one checkout and restored by copy leaves an artifact that the
   other checkout's mtime fingerprint accepts, so the verify run failed `bundled_generators_match_the_golden_baseline`
   with the mutated name (`hole_size` not declared as a number). Touching the source and rebuilding made it
   pass with identical source. Mutate only in the checkout whose test you run, or touch the files after.

Commands and observed results:

- `python3 scripts/check.py repo lint` in the worktree: passed (repo 0.1 s, lint 8.6 s).
- `python3 scripts/check.py test typecheck` in `/tmp/claude-1000/boardstudio-tce-verify` with the verified
  files copied over `6cc81e4ba`: `all steps passed: test, typecheck`, exit 0, test 105 s, typecheck 29 s;
  summed results 1325 passed, 0 failed (1316 before: +7 lint tests, +1 wiring writer test, +1 web
  catalogue test). `parameter_lint`: 15 passed. Both Core writer tests: passed (about 70 s each in a debug
  build, the projects are built once).
- Mutation checks of the new lint: 18 mutations of its rules, exceptions and name helpers, and 2 of the
  pinned counts, each applied alone and reverted; every one fails a test (the one survivor, the
  label-stem pad-name check, got a test and then failed).
- `cargo clippy --manifest-path footprints/Cargo.toml --all-targets --no-deps -- -D warnings`: no issues.

GitNexus: the index is stale (indexed `4b372b2`, current `0c0de4714`). `detect-changes --scope all` reported
risk low, 0 processes, with only doc sections and the `physical_setup.rs` test module as symbols; it does
not map the other Rust test files, so it is incomplete. No production symbol was edited (`git diff
0c0de4714..HEAD -- core/src footprints/src` is empty); the only edit to an existing file's code is a new
`#[test]` in `physical_setup.rs`, so `impact` had no symbol to run on.

Not verified: `build`, `browser`, `deny`, `quick` and `release` (no wasm-visible or dependency change; `release`
by rule). The lint still sees only declared names and defaults. Saved documents outside this repository
may hold numeric text or undeclared names. The index is stale.
