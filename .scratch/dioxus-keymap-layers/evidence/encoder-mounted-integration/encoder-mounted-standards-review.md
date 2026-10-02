# Encoder root mount — Standards review

Reviewed integration working-tree mount against HEAD `7a165a8ecdb1991bd12fd71bcf7ef2e0f66e2740`, plus the new private input helper. Exact SHA-256:

- `web/src/presentation.rs`: `f52d368f790a7f644a969610b8a24d5c866d09585bc3ee0066c4eb75e87d59d5`
- `web/src/presentation/keymap.rs`: `6ed26fd5537de078fed9255be720476171e42e286b2354fc3eaf1989947e93f4`
- `web/src/presentation/keymap/encoder_inputs.rs`: `490435cba8c1ffc54923406eddf3934be6eebb4b40c1c86700e121efed76a728`
- `web/assets/m1.css`: `79f2e4c7d1b25664a75b96f0508f681c9898738f2cdc473952d10e35587a39da`

No material Standards findings in this bounded mount. Both input and operation hooks run before Editor’s conditional returns. Their Editor lifetime retains request sequencing and operation observation while panels hide. Encoder rows mount inside the existing Keymap panel alongside ordinary bindings and macros, independent of ordinary selected-key availability. Root component keys include full Scope, effective layer, editor identity, and input lineage; binding-only token/revision advances preserve drafts.

The helper retains one accepted-source cache, immutable shared row handles, and one lineage comparison value. It reprojects on Scope/token/revision changes, not ordinary rendering, and its live getter checks the current Runtime snapshot. It uses Core’s public peripheral description and real reported push IDs; attached module rows remain visible without pretending Core accepts their edits. The corrected `model::RotaryProfile` import uses the existing public reexport. No document/keymap cloning, domain authority duplication, or library API widening was added.

CSS targets the actual encoder class, uses existing theme tokens and panel control styles, and provides 44px compact disclosure targets. No unrelated workspace rules changed.

Root subsequently reported the cleared controller cleanup cherry-picked as `2759ce03`, with strict WASM all-targets Clippy, formatting, and whitespace checks passing; this resolves the inspection-time grouped-caller dependency. No Cargo or browser checks were performed by this reviewer. Root’s 34 native tests do not establish execution of encoder-specific presentation tests. Actual encoder interaction, module rejection/retry, layout/accessibility, and the absent full F5 fingerprint/handoff remain separate gates.
