# Root encoder integration: independent Spec review

**Source clear for the bounded mount; no material findings.** Reviewed dirty integration against HEAD `7a165a8ecdb1991bd12fd71bcf7ef2e0f66e2740` and previously cleared controller grouping. Exact SHA-256:

- presentation.rs: `f52d368f790a7f644a969610b8a24d5c866d09585bc3ee0066c4eb75e87d59d5`
- keymap.rs: `6ed26fd5537de078fed9255be720476171e42e286b2354fc3eaf1989947e93f4`
- m1.css: `79f2e4c7d1b25664a75b96f0508f681c9898738f2cdc473952d10e35587a39da`
- encoder_inputs.rs: `490435cba8c1ffc54923406eddf3934be6eebb4b40c1c86700e121efed76a728`

The input hook remains unconditional in Editor and precedes conditional workspace/Inspector rendering. BindingProjectionSources receives the accepted LayerSource/canvas projection, separate display Memo, and synchronous current getter. Ordinary selected-key bindings retain their existing mount key and receive explicit Key target with no encoder identity.

EncoderEditor mounts inside the Keymap panel children independently of selected-key presence. Its key includes full render Scope, effective layer, stable input generation and editor lifetime, excluding admission token/revision. Genuine input changes reset drafts; binding-only saves preserve them. Shared enabled/feedback/sequence/callback values stay with the reviewed controller. Module rows use the React host-board/catalogue/profile rule without fabricated push IDs; physical push IDs come directly from Core. The helper’s only delta from prior clearance is the correct existing model re-export for RotaryProfile.

CSS targets the actual widget classes, keeps rotation summaries visible with 44px compact targets, and uses existing theme tokens. Actual disclosure, focus and compact layout require public verification.

**Open limits:** current physical rows use accepted Core descriptions, not a completed F5 resolved-plan handoff; absent plan fingerprint remains explicit. F5 readiness/order/source replacement and firmware output acceptance remain open. This source review does not establish compiled compatibility before the controller cherry-pick, successful own-revision acknowledgement, module rejection recovery or public save/reload/Undo. Root compilation and fresh browser regression are required. No source edits or Cargo performed.
