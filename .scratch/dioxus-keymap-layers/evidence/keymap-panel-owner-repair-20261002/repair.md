# Keymap selected-owner query repair — 2026-10-02

Defect correction, isolated `codex/keymap-search-owner-repair-20261002`, base4bbafae0. Root authorized Astra repair after paired candidate34722 / fresh React5173 evidence. No root checkout/source or other author's files edited.

## Confirmed behavior and cause

The auditor's minimized same-archive trace: searchSW17, selectSW17 in the chooser, then selectSW18 with canvas focus/Enter. React immediately clears query on selection and the chooser showsSW18. Dioxus retainsSW17, heading showsSW18 but the native chooser is blank because its only nonempty option isSW17. The earlier longer-session claim that Macros→Keys itself selectsSW1 was withdrawn. Tabs alone preserve search in the reference.

Independently reproduced the candidate failure in an isolated browser session on34722 using a fresh Sofle copy: `browser-before.json` records SW18 heading, empty native selected value, SW17 query and onlySW17 option. This separate-copy observation is not a claim of the auditor's same-archive fixture provenance.

Actual React `app/src/ui/Workbench.tsx:1399` keys Inspector content by board, scope and activePart identity; `KeymapPanel.tsx:13–14` owns query and section inside that keyed boundary. Selection changes therefore reset both presentation values. Dioxus's actual KeymapPanel had persistent unscoped query/editor signals.

## Correction

The existing panel now reconciles query/editor state on `(Scope, selected_key_id)` changes with a `use_reactive` effect. Changed-only writes and `peek()` bookkeeping avoid subscribing the effect to query/tab values. No reset runs for same-owner typing, tab changes, layer changes or operation feedback. Shared Session selection remains authoritative; the effect emits no selection/document operation and no public API/schema/dependency changes.

Production source SHA256: `3d2dbab1670adab446d2de69e9fa989766d2cb2971e1aeeb48c5ddb578424cb7`.
Mounted harness SHA256: `b82a6690783f90c11e97aa896302d75a018e0e7e83451e35517126339b60ff08`.

## Verification

`web/tests/keymap_panel_lifecycle.rs` imports the actual production panel, view projection and layer request types. A native VirtualDom drives real HTML form/tab callbacks against the accepted projection; externally accepted canvas selection is delivered through the same selected-key prop. DOM mutations expose rendered input/select values and tab lifecycle. It does not pretend to run browser Session/Core or actual canvas hit testing.

Old production source: four expected failures (chooser query reset, canvas-hidden current key, same-ID/new-session query, new selection returning from Macros to Keys), with two preservation tests passing. Corrected source: all6 pass. Logs `red.log`, `green.log`. No duplicated test-only reset policy.

Commands: `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page --test keymap_panel_lifecycle`; strict `cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings`; fmt check and git diff check all pass. Reused idle PCB06 target directory for compilation only. Native executable build reports seven pre-existing unused provider/archive items whose consumers are WASM-only; no suppressions added. Strict WASM passes (`wasm-clippy.log`).

Independent Spec/Standards review, serial integration and fresh release-browser replay remain required. No ticket/parent/full Keymap closure or fixed-browser result is claimed. Wider keymap acceptance remains open.

RF handoff: RF-006 — presentation query/editor lifetime must follow accepted selection scope without becoming selection authority. RF-009 — retain the minimized trace and explicit withdrawal of the longer-session tab/reset interpretation; distinguish old-browser red, native fixed proof and future packaged green. No new RF identity proposed.
