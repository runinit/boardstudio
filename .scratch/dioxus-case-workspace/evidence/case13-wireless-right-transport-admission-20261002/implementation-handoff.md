# Case13 Right wireless transport admission handoff

Date: 2026-10-02
Branch: `codex/case13-wireless-right-20261002`
Base: `61a81cc2bbaa7546e6e1452d3bf7b3735cd4f365`
Scope: private Case physical-setup action ownership in `web/src/presentation.rs`.

## Confirmed behavior and diagnosis

On the exact layered Sofle fixture, select the Right PCB/half, open Case → Assembly setup, and change Half connection to “Wireless · local battery on each half.” The candidate selector displayed Wireless while the adjacent guidance remained the wired TRRS text; the wired-only unchecked “Include a battery envelope” control stayed visible; the app remained Saved at revision 21. Export was byte-identical to the pre-selector archive, with `hardware.transport = wired`, revision 21, and no Right mechanical battery. Reload reset the selector to Wired. The selection was therefore not an accepted operation and was silently rejected.

QA evidence is retained at `/home/chris/.local/share/boardstudio/retained-tmp/20261002/case13-public-qa/report.md` and `payload-comparison.txt`; the original candidate screenshot is `/home/chris/.local/share/boardstudio/retained-tmp/20261002/case13-public-qa/screenshots/candidate-wireless-right-include-visible.png`. The source fixture archive SHA-256 is `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`. The unchanged pre-action/export archive SHA-256 is `9d906360d185ccab53957b714647a6978afc137bb28f4d8e9c38bad307f9c8ab`.

The owner predicate required a selected tree context even though the transport control belongs to the active Case assembly scope. Selecting Case clears that context, so the visible control's `CaseTransport` action was rejected by `PhysicalSetupMount` admission. This is an admission/ownership defect, not a battery projection defect: importing the genuine React Right-wireless-and-flipped archive (SHA-256 `430657c3d1aafeb719c80988dfc9cf62c99042b5ef009936e4ef9848e8e35ed2`) into Dioxus correctly hid the Include checkbox and rendered the wireless guidance and all eight fields with reflected values. React's same Right wireless action accepted `hardware.transport = wireless` and showed the eight effective fields; its retained capture is `react-wireless-right-fields.png` in the QA screenshots directory.

## Correction

`case_setup_context_is_current` now admits the Case owner by the accepted Case workspace and captured session/document/board/instance scope, with strict instance matching preserved. The owner path still uses the surrounding generation, token, revision and current-instance checks. ProjectGuide admission is unchanged. No public API, schema, reset behavior, or unrelated normalization was changed.

## Regression and checks

The focused browser-WASM regression `case_transport_owner_does_not_require_a_part_tree_selection` reproduced red before the correction: it asserted that physical setup belongs to the active Case assembly scope without a selected part, and failed that assertion while the old selected-context prerequisite remained. After removing that prerequisite, the same command passed 1 test:

```text
wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- case_transport_owner_does_not_require_a_part_tree_selection
1 passed; 0 failed
```

The current final source also passes:

```text
cargo fmt --manifest-path web/Cargo.toml -- --check
cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web -- -D warnings
git diff --check
```

The strict Clippy output is preserved in `clippy-green.log` (SHA-256 `8b1fd2394f7b6f177ede882431154a419bd9f404f00add314c912e91a7fc0ec0`). The focused post-fix browser-WASM output is preserved in `owner-green.log` (SHA-256 `45ef24ba36f92437a567d5ada3def9b3a7cc0decc8917666109656271b733e5f`); it reports 1 passed, 0 failed. The test result completed, though the shared runner needed an interrupt during cleanup and therefore its shell exit was 130. Rustfmt, JSON parse and diff checks pass. The original red output was observed during implementation but was not preserved to a file, so there is no red-log path/hash to report. Independent Sol review of source commit `6dfb7fe4` is clear in `/home/chris/.local/share/boardstudio/reviews/case13-transport-source-review-6dfb7fe4-sol-20261002.md` (SHA-256 `bbbb1280cc808f1997d46bf468a3d0ae8ecbd7fc9f36ff9d6761c0fe8808dd34`). This isolate has not yet been rebuilt into a fresh public candidate, and the post-fix browser repro, accepted archive persistence, Undo/Redo, and reopen journey remain open for independent review/verification.

## Review and RF-006 disposition

This is a bounded confirmed selection-scope/assembly-scope ownership observation under existing RF-006. The machine register and `docs/migration/POST-PORT-REFACTOR.md` were updated; no new RF ID or structural proposal is introduced. Independent Sol source review and a fresh public post-fix run remain required. No parent acceptance is claimed.
