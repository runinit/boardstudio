# Pinned Dioxus listener repair proposal

The coordinator approved this bounded repository-owned repair after reviewing `8c42a75` and
allocated the exclusive prerequisite build slot. Shared prerequisite landing remains held. Keep
the Button effective-id and sorted-spread corrections; neither is replaced by this renderer repair.

## Executed evidence and root cause

The mounted sorted Button listener update still fails in a fresh browser, while its native
mutation test emits one focus removal per native/link branch and no blur removal. The evidence
and earlier source pins are in [the prerequisite note](shared-control-prerequisites.md#additional-pinned-renderer-blocker-after-sorting).

Three renderer invariants are broken in the published `dioxus-interpreter-js` 0.7.10 source:

1. `unified_bindings.rs:86-87` removes `data-dioxus-id` before native listener cleanup. Nonbubbling
   cleanup reads that same attribute and indexes its local listener table with null.
2. It removes the routing id on *every* event removal, even while `node.listening` remains positive.
   Both Dioxus web dispatch (`walk_element_for_id`) and native dispatch (`getTargetId`) need that id
   to route retained handlers to the correct element.
3. `core.ts:124-128` creates an empty local record but never records each registered event/callback.
   After preserving the id, the first local removal still deletes that empty record; a second valid
   local removal then fails. Hydration and the binary-protocol registration path call the same
   registration function. [Pinned binding source](https://github.com/DioxusLabs/dioxus/blob/57d6794ad60b949e5bd8aa282f6f8c3dc97a365e/packages/interpreter/src/unified_bindings.rs),
   [pinned TypeScript source](https://github.com/DioxusLabs/dioxus/blob/57d6794ad60b949e5bd8aa282f6f8c3dc97a365e/packages/interpreter/src/ts/core.ts).

Read-only, in-memory Node probes exercised the actual published `core.js` and the Rust binding's
embedded registration/removal statements. The pinned code throws; a reorder-only candidate loses
the retained handler's id; ordering plus a last-listener count check still fails on the second
nonbubbling removal. The proposed three-part repair passes exact-detach, retained-blur, mixed
local/global listener counts, last-handler id cleanup, re-add and hydration probes. Evidence:
`.evidence/shared-control-prereq/renderer-listener-invariants-probe-v3.log`. This is a Node EventTarget
unit probe, **not** a browser/WASM pass. Initial probe adapter failures (cross-VM TypeError identity,
then an invalid textual replacement of minified syntax) are retained separately; they are not
product REDs. No candidate source was written to the Cargo registry or cache.

## Smallest repository-owned dependency mechanism

Use the existing root `[patch.crates-io]` plus excluded-vendor pattern already used by cgmath:

- Add `web/vendor/dioxus-interpreter-js-0.7.10/` containing the published crate (32 cached files,
  about 212 KiB), its upstream license notices and a short `PROVENANCE.md` recording version,
  upstream commit `57d6794ad60b949e5bd8aa282f6f8c3dc97a365e` and the local delta.
- In root `Cargo.toml`, add that path to `workspace.exclude` and add
  `dioxus-interpreter-js = { path = "web/vendor/dioxus-interpreter-js-0.7.10" }` to the existing patch
  table. Preserve the package version, feature declarations, dependency constraints and public API.
- In `Cargo.lock`, only this package should change from registry identity to path identity. Reject
  unrelated package/version/source changes. The current lock has exactly one reverse dependency,
  `dioxus-web` 0.7.10. Its interpreter features are `minimal_bindings` and `webonly`; no desktop or
  liveview interpreter reverse edge is present. All resolved Dioxus framework packages stay 0.7.10.

Only these vendor implementation files need a hand-authored repair:

| File | Bounded change |
| --- | --- |
| `src/unified_bindings.rs` | Detach the requested listener while the id still exists; decrement the active count; remove the id only when the count reaches zero. |
| `src/ts/core.ts` | Record the actual callback under its local event name at registration; remove that recorded callback and only that table entry at removal. |

Regenerate `src/js/core.js` and the crate's existing `src/js/hash.txt` through its unchanged
`lazy-js-bundle`/Bun build mechanism. Bun 1.4.4 is installed. The other published JS bundles do not
contain `BaseInterpreter` or the failing removal routine; inspect generated output for unrelated
changes before committing. The existing hash is upstream build metadata, not a new evidence
manifest. No global registry/cache edit, blanket Dioxus upgrade, feature broadening or control-local
listener history/None-slot workaround is proposed.

The current `removeAllNonBubblingListeners` clears the element's local bookkeeping when existing
remove/replace paths detach its DOM node. Preserve that policy; this repair does not refactor
descendant teardown or claim to solve unrelated detached-node retention. Registration from
hydration and foreign/binary listeners must use the same corrected table/count invariants.

## Upstream check and verification gate

The official newest-five release listing inspected on 2026-10-22 contains v0.7.10 and 0.8 alpha
releases, not an identified compatible 0.7 successor. Upstream commit
`24f6a829df0dfa203961a98ea4cae21c2ff27e28` moves removal behind detach and guards routing-id cleanup
by count, but also replaces the interpreter protocol in a 272-file change. Its local registration
table still needs the separate event-recording analysis above. This is not a justified single-crate
upgrade candidate. [v0.7.10 release](https://github.com/DioxusLabs/dioxus/releases/tag/v0.7.10),
[upstream protocol change](https://github.com/DioxusLabs/dioxus/commit/24f6a829df0dfa203961a98ea4cae21c2ff27e28),
[upstream removal implementation](https://github.com/DioxusLabs/dioxus/blob/24f6a829df0dfa203961a98ea4cae21c2ff27e28/packages/interpreter/src/ts/core.ts).

After approval and build-slot allocation:

1. Commit the vendored baseline and a small Node regression against its actual JS/binding source
   before applying the repair; demonstrate the published-code RED without a cache mutation.
2. Apply the three-part repair in a separate commit and require the same unit probe GREEN. Cover
   two nonbubbling handlers removed in sequence, retained local and bubbling handlers, another
   element retaining a global subscription, last-handler cleanup, re-add, hydration, and the
   existing element-teardown bookkeeping entry point.
3. Require the mounted native/link Button and InputGroupInput transitions GREEN, including callback
   count/order, retained blur, routing identity, unknown-attribute removal, collision policy and
   precise caller value. Re-run all shared `ui::` WASM tests and the affected native tests/Clippy,
   format, documentation and test-name guards on the exact combined source pin.
4. Audit the resolved path/feature/lock closure, then request **one completed-correction two-axis
   review** through the coordinator, explicitly covering renderer Standards/Spec and shared-contract
   P2 resolution together. Landing and global acceptance stay held until those gates pass.

## Authorized implementation receipt

Published baseline and unchanged regression: `9ee203f69eaf1ace39f5f5b5e1fa88133074251a`. Every one of
the archive's 31 files was compared byte-for-byte with the checked-in baseline; the earlier count
of 32 cached files included Cargo's `.cargo-ok`, which is not published source. Upstream license
texts, provenance and the Node test are additions. Root `Cargo.toml` uses the approved patch/exclude
pattern; the root lock delta removes only the interpreter package's registry source/checksum.

`node web/vendor/dioxus-interpreter-js-0.7.10/tests/listener-removal.cjs` failed against that published
baseline: **1 passed, 4 failed**, with the actual null-key TypeError on callback removal and missing
event records on teardown. `.evidence/shared-control-prereq/vendor-listener-red.log`. The test was
committed before the hand-authored renderer repair.

Renderer repair: `d29dec86553a8cd532c4eb764c07a80e39ec0e2d`, tree
`964d4115c141c529ea690b968875d7bdf1f9222b`. Its entire vendor delta is the two approved source files,
the generated `core.js`, and the existing hash file (only the core TypeScript input hash changes).
The same unchanged Node regression now passes **5/5**;
`.evidence/shared-control-prereq/vendor-listener-green.log`.

The unchanged build mechanism ran through
`cargo check --locked --offline -p boardstudio-web-ui-shared --target wasm32-unknown-unknown`, which
passed; `.evidence/shared-control-prereq/vendor-bundle-compile.log`. Bun 1.4.4 also rewrote three
unrelated outputs (`hydrate.js`, `native.js`, `patch_console.js`) through export ordering, local
renaming and destructuring. Inspection showed no dependency on the repaired core implementation;
those three files retain their published baseline bytes. All other published files and the bundle
build script are unchanged. Combined mounted/native checks and the single correction review are
pending.
