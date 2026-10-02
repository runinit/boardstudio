# Matrix and Keycaps integration wave — 2026-10-02

Scope: partial frontend parity progress, not parent acceptance. User concurrency ceiling is 30; this session provides 11 slots including coordinator. Six persistent workbench queues and all canonical 62 parent criteria remain.

## Built source and executed checks

- `23214cc4`: strict WASM Clippy (all targets, warnings denied) passed after retained compile/lint failures and narrow corrections. Native page tests: 12 library + 22 binary passed; these do not execute WASM-only presentation tests.
- `92db0b8f`: final rustfmt passed after module-order formatting. Build `frontend-matrix-keycaps-20261002` completed all 8 packaging commands and verified 983 source hashes. Public demo: http://127.0.0.1:34691/ and `/boardstudio/`.
- `a896cd49`: CSS-only Matrix form styling correction after actual browser found unstyled inline fields. Build `frontend-matrix-keycaps-styled-20261002` completed all 8 packaging commands and verified 983 source hashes. Current demo: http://127.0.0.1:34693/ and `/boardstudio/`. Root visually inspected the styled screenshot.

## Public journeys

Same portable Sofle archive SHA256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`, imported through public UI in separate profiles. React reference remains `5a472a9426e6e38993361da402cd4ec730feb369` at5175.

- Matrix: select Layout `keys`; name change accepted at revision10; Undo restored at11; Redo at12. Rows4→3 accepted at13, part count70→58 as in React; reload preserved name and rows3 at saved revision13. This demonstrates real editing and persistence, not the complete matrix-authoring feature family.
- Styled Matrix: entering Rows0 then Enter shows `Enter a positive whole number.` with saved revision9 unchanged; Escape restores Rows4 and clears the error. Other context/stale-operation, compact, pitch, and full parent criteria are not inferred from these checks.
- Keycaps: actual29-key 2D scene, contour, physical legends/colors and selected-key summary mounted. Available matrix/key lists were moved to the right Inspector after direct reference comparison; settings/3D/full profiles remain subsequent work.
- Keycaps original strict-reference red (retained): after selecting the Layout matrix then clicking SW7 in Keycaps, React retains matrix selection and reports SW1 as its first selected key; Dioxus selects SW7 alone. Both settled paths are retained; Astra is diagnosing the shared selection policy. No Keycaps selection-parity or F6C completion claim.
- Keymap: independent green verification of the dropdown remount fix lives under `/tmp/frontend-parity-reset-20261002/keymap/green-92db0b8f/` and its separate durable handoff. It does not close the entire workbench.

## Refactoring takeaways

RF-001/RF-009: feature composition source review did not establish visible placement or stylesheet integration; browser comparison found both the misplaced Keycaps controls and the unstyled Matrix form. Both bounded defects are corrected and history retained. Defer the wider shared composition/visual contract redesign.

RF-005/RF-009: actual shared selection context differs between TypeScript and the first Keycaps mount. The current bug requires parity repair; a broader unified selection policy remains a post-port design question. Retain red evidence rather than treating a typed guarded callback as behavioral proof.

PCB, Parts and Case first-source slices are reviewed in isolated workers and still require integration/build/browser acceptance. Follow-on Layout/PCB/Keycaps contracts are being refined without deleting parent blockers or historical rationale.

## Selection discrepancy resolution

Independent paired/source diagnosis confirms a legacy React stale-closure bug: its explicit Key-mode reset is overridden by the same-render callback still reading Matrix mode. Dioxus single-key selection is retained under the user-confirmed Q1 exception for confirmed bugs. The intended single-key oracle passes Dioxus and fails React; choosing Key mode first makes React pass. See [diagnosis](keycaps-selection-diagnosis/DIAGNOSIS.md). Original red proof remains; this is an explicit justified deviation and does not close F6C.
