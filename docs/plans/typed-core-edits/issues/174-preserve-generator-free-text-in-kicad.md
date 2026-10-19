# 174: Preserve quoted generator labels in valid KiCad exports

Status: resolved
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [Input-boundary audit research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

Users can enter ordinary quotes, backslashes and Unicode in supported generator
text/label fields and get the same text in a valid KiCad board. The preview and
manufacturing export must agree; requiring users to pre-escape their saved text
is not an acceptable fix.

The audit entered a label containing an inch mark. Packaged Core returned a
successful board export whose text was split into a shortened string and an
extra token. KiCad 10 accepted the plain-text control and rejected the quoted
label. Both text generators and active labels in the pad-row and SMD generators
reproduced the loss.

## Acceptance criteria

- [x] First reproduce exact text loss and invalid KiCad export with the inch-mark
  case. Prove the corrected parsed text equals the user's original value.
- [x] Quote/escape literal text once at emission using the existing serialized
  string representation. Inventory affected generator free-text interpolations,
  including active labels and any affected font/reference literals; apply the
  same correction without escaping atom selectors or route syntax as strings.
- [x] Cover both text generators and active front/back/reversible labels in pad
  and SMD output. Native cases include quotes, backslashes, Unicode and supported
  whitespace; literal escape-looking text must not silently change meaning.
- [x] Saved parameter values stay literal and compatible, with no double escaping
  through rendering, normalization, preview or export. Unrelated nets, model
  references, numeric geometry and ordinary generator golden cases stay intact.
- [x] Fresh packaged Core exports containing the affected text pass a real KiCad
  load check, alongside a plain-text control. Assert exact emitted label values
  as well as parser acceptance so silently deleting the quote cannot pass.
- [x] Run affected generator/Core export checks. Keep physical hardware
  qualification and non-text compatibility decisions unchanged; do not refresh
  broad goldens to hide unintended output changes.

## Comments

Created from the user's audit findings after primary-source research. Independent
of 175's PCB import identity correction. Use a narrow literal-escaping correction,
not a new generator language or wholesale rewrite of generator templates.

### Reverified after the footprint-engine merge (2026-10-19)

Merged `footprint-engine` at `2fe19cf42` into `dev`. The new `Body::Forms`
path currently ports only `ceoloide/mounting_hole_npth`; both text generators
and the affected pad/SMD generators still use `Body::Text`.

A temporary integration probe exercised `bundled().render` for
`ceoloide/utility_text` and `infused-kim/text`, asserting that the emitted
`gr_text` string equals the literal saved `text` parameter. Both plain controls
(`Board 12 inch`) passed; both inch-mark cases (`Board 12" inch`) failed with
actual `Str("Board 12")`. Command:
`cargo test -p boardstudio-footprints --locked --test ticket_174_probe`.
Log: `.evidence/174-after-footprint-engine.log` (local, gitignored).
The temporary probe was removed after diagnosis; no production fix was made.

174 remains needed. This is fresh generator-boundary evidence, not a fresh
packaged Core export or real KiCad load check. The new structured-output path
is available to the implementation; preserve the acceptance criteria above.

The merged footprint crate's full suite passed with
`cargo test -p boardstudio-footprints --locked`
(`.evidence/footprint-engine-merged-tests.log`). Separate Standards and Spec
reviews of `b4d788c7c...f57e3a4e1` reported no findings for the narrow merge;
neither treated this first structured-output port as a fix for 174.

## Outcome

Implemented on `dev` in commits `0ea5b26d5` and `3d77c263c`.

Generator free text is serialized once with the existing JSON-compatible
`json_string` helper at emission: both text generators (`text`, font `face`,
text `layer`), pad-row and SMD labels, active MCU via/silk labels, quoted
`Reference` literals across the bundled generators, the ceoloide molex model
filename, and filled-zone/keepout names. At the two sites that historically
wrote a reference as a bare atom, `atom_or_string` keeps token-safe ASCII bare
(so goldens are unchanged) and quotes anything else. Atom selectors and
`utility_router` route syntax are untouched; saved parameter values are passed
through unchanged.

Core's generated-form canonicalization decoded quoted escapes and then wrote
control characters literally. `escape_quoted_controls` now emits `\n`, `\r` and
`\t` escapes (and rejects other control characters), and board objects keep the
validated canonical form instead of parsing it lossily a second time. The same
decoder now reads JSON `\uXXXX` escapes, so unsupported control characters are
rejected instead of becoming literal `\u0001` text. Preview text uses the raw
source span decode, so preview whitespace and labels agree with export.

Verification:

- RED: `.evidence/174-red.log` shows the inch-mark labels truncated to
  `Str("Board 12")` at both text generators, pad-row labels and SMD labels.
  GREEN: `.evidence/174-green.log` covers quotes, backslashes, Unicode,
  newline/tab whitespace, literal escape-looking text, front/back/reversible
  output, and bare-vs-quoted references.
- `.evidence/174-footprints-suite.log`: all 17 footprints test binaries pass
  (including the golden baseline, 820 recorded cases).
- `.evidence/174-core-lib-suite.log`: 797 core lib tests pass, 4 ignored.
  `.evidence/174-core-generator-export-suite.log`: 10 generator export tests
  pass. `.evidence/174-kicad-integration.log`: 23 KiCad integration tests pass.
  `.evidence/174-clippy.log`: clippy with `-D warnings` passes for both crates.
- Fresh packaged Core WASM (`wasm-pack build --dev --target web --out-name
  m1_core_worker --no-default-features --features core-worker`) exported the
  same quoted/backslash/Unicode/newline labels through `artifact_request`.
  `.evidence/174-packaged/probe.json` asserts 5 exact `gr_text` values and 6
  exact labels, including literal `\n` text and newline/tab whitespace;
  `.evidence/174-kicad-probe/packaged-load.log` records KiCad 10.0.7 accepting
  that board (exit 0) with a plain-text control in the same board. The native export control is also asserted in
  `generator_export::exports_quoted_generator_text_and_active_labels_for_kicad`.
- Preview/export agreement is asserted in the native export test; a preview
  unit test covers newline/tab and quote/backslash decoding, and the generated
  `utility_logo` layer parameter is covered in the footprints literal test.
- KiCad CLI had to run with the T3 Code AppImage's `APPDIR`, `LD_LIBRARY_PATH`
  and related variables unset; otherwise the launcher looks for
  `_pcbnew.kiface` inside the stale AppImage mount.

Limitations: display labels derived from net names remain a separate net-name
boundary and were not re-serialized here. Control characters other than
newline, carriage return and tab are rejected during generated-form export
rather than given an unverified KiCad escape.
