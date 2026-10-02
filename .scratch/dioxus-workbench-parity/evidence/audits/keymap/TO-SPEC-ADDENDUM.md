# Proposed Keymap parity addendum (review draft; no source edit)

User decision recorded for this stream: preserve the pinned TypeScript UI's placement, hierarchy, labels, and interactions except for an explicitly reviewed bug/platform difference. Keep the current specification's no-new-public-API and existing-provider constraints.

## Acceptance wording to add/reconcile

1. **Composition:** The mounted Dioxus Keymap inspector should present the same `Keys` / `Macros` / `Encoders` editor navigation group and one active editor surface as the pinned React `KeymapPanel`, with matching placement, label, selected state and default section. Do not treat the presence of all three stacked subpanels as an equivalent substitute. Any proposed alternative needs an explicit user-approved parity exception.
2. **Board outline:** With the same accepted selected board and outline input, Keymap workspace navigation should expose the reference `Outline` / `Generated` tree structure and its generated contour in the Keymap canvas. This is a presentation/read-model handoff acceptance item; use existing outline/core data and shared tree/canvas ownership. Do not create a second outline model or imply Core capability is missing.
3. **Export:** If Issue 08 is demonstrated, the existing Keymap `Export ZMK source` action must reach the existing qualified firmware provider and browser-download path, retaining current readiness/qualification errors. No UI-local DTS generation or new command/API. Keep F5.2 and F8.2 joins explicit.
4. **Acceptance layering:** Issue 01's paired fixture should assert projection, layer and key labels, selection/no-match behavior, composition, and outline handoff. Keep the F3.1 shared-selection join open until shared selection is actually exercised. Issues 03/04/06 remain responsible for their own behavior matrices; this addendum does not close any ticket or parent.

## Ownership mapping for review

- Composition/tab navigation: mounted Keymap panel owner/coordinator; coordinate existing F6K.1/F6K.2/F6K.3/F6K.4a module ownership.
- Outline tree/canvas: shared tree and Keymap canvas/read-model owners, with F3 outline integration as source of existing outline data.
- ZMK action/provider: Issue 08 owner plus coordinator; F5.2 selected-board electrical handoff and F8.2 delivery remain acceptance joins.

This draft records observed public parity gaps only. It does not add graph edges, change canonical gates, or prescribe implementation internals beyond already-stated ownership and API boundaries.
