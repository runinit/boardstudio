# F6K.3a: Use display names for macro step controls

**Parent:** F6K.3 — Structured macro editor (`.scratch/dioxus-frontend-v1/tasks.json`; `.scratch/dioxus-frontend-v1/issues/06-keymap-keycaps.md`).

**Source specification:** [`09-macro-step-accessible-names-spec.md`](09-macro-step-accessible-names-spec.md).

**What to build:** Make macro step controls identify themselves using accepted user-facing macro/field labels, matching the pinned React editor, instead of exposing the generated stable macro ID to assistive technology.

**Capability gate:** F6K.3's mounted macro editor and private typed edit path already exist. The accepted macro display name, stable identity and step sequence are available to the macro card/editor. This ticket changes only labels on that existing surface; it does not require a new provider, Core operation, public API, or owner.

**Acceptance:**

- [ ] Give each step-kind selector the accessible name `<accepted macro name> step <1-based step index>`, matching React. Update the label after a name edit is accepted and when preceding steps are added or removed.
- [ ] Associate the non-wait keycode input with the visible `Keycode` label and the wait duration input with `Delay (ms)`, matching React. Do not include the macro ID in either accessible name.
- [ ] Keep generated macro ID, target step, scope/source envelope, typed edit intents, validation, history and visible editor behavior unchanged. Accessibility labels must not use unaccepted name drafts.
- [ ] Add a production-used regression seam for rendering/deriving these labels and verify accepted rename plus step-index changes. Confirm intents still target the same stable macro ID and step.
- [ ] Compare actual React and Dioxus accessibility snapshots using the same saved Sofle fixture: default Macro 1/tap-A, accepted rename, add/remove step, wait and return to tap. Assert exact names and no internal-ID leakage.
- [ ] Run the affected native/WASM tests, strict WASM Clippy, formatting/diff checks, and the paired browser check on the changed candidate source. Record source/build/fixture provenance.
- [ ] Preserve RF-009 source accounting. This child does not close F6K.3, F6K.1, F6K.2, F3.1, INT.2 or any parent acceptance join.

**Ownership:** Existing F6K.3 macro editor owner; no shared composition, runtime, Core, storage, or public contract changes are expected. Coordinate with the current macro owner before touching the editor module.

**Profile:** Luna Medium author/verifier; Sol 6.1 High independent source/spec reviewer. This bounded view-label change uses the already mounted editor/accepted source and introduces no asynchronous owner or edit path.

**Status:** Draft for independent review; not published or counted.
