# 01: Inspect and select keys in the Keymap workspace

**What to build:** The Keymap workspace shows the selected board’s supported key set and active layer, lets the designer select a key on the canvas or from the searchable Selected key control, and presents existing binding labels without editing them.

**Blocked by:** INT.1 is the parent start edge. The coordinator must prove a Keymap-specific page mount and accepted-document, board-scope, shared selection and callback path; INT.1 does not provide a Keymap contract. F3.1 remains an acceptance join for shared selection/read-model integration.

**Status:** ready-for-agent

- [ ] Project a virtual Base map when keymap data/layers are absent; resolve active layer by stable ID with first-layer fallback and retain legacy base `keyBinding` semantics. Ordinary legacy boards are not misreported as empty/unavailable.
- [ ] Key membership matches the reference: switches and supported direct matrix key parts are included; matrix companions are excluded. Stable key IDs, layer order/names and base default agree with fixtures.
- [ ] The canvas and native searchable Selected key select share existing selected-part selection; option labels show reference plus existing binding title, including transparent/none distinctions and transparent fallback for absent non-base bindings.
- [ ] Viewing/selection/query state does not change revision/history or edit bindings. Empty actual key sets and search no-match have accurate UI states.
- [ ] Paired public Dioxus browser evidence covers legacy/default, layered, canvas/list selection, active-layer labels and scope changes; F3.1 acceptance join stays open until its real shared handoff is verified; record RF observations.

- [ ] Complete inherited shared acceptance, independent Standards/Spec review and RF handoff; parent acceptance/joins remain open.
