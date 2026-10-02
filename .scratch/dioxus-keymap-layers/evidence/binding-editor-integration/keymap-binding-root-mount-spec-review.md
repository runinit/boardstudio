# Keymap binding root-mount Spec review

**Source-clear after the search correction.** Read-only review of integration HEAD `86a83ddfee611899695de0952c9996794dd12e41` plus released dirty mount/compiler changes. No Cargo or browser run by this reviewer.

**Resolved P2 — Search must not unmount the selected editor** (`web/src/presentation/keymap/panel.rs:121–126`). Issue03 requires a rejected draft to remain “visible and editable until its target/value changes or the user corrects it.” The new `{children}` mount is beneath `else if no_matches`; an unmatched Find a key query therefore removes the selected key’s editor despite unchanged Scope, layer, key, editor identity and accepted value. After a validation failure, searching and clearing that query loses the recoverable draft. Pinned React `app/src/ui/KeymapPanel.tsx` filters select options independently and renders its selected KeyBindingEditor regardless of query. Keep the selected editor mounted and render the no-match status separately, or restrict it to the no-selection branch. The corrected selected-key branch now precedes `no_matches`, renders the search status inside that branch, and keeps editor children at their stable position. This clears the source finding; public red/green proof remains required.

Otherwise the root hook is unconditional before accepted-state early returns; it shares the accepted projection, cloned LayerSource and Editor-lifetime request sequence. The editor key contains Scope/effective layer/key/editor identity and excludes token/revision. Fresh snapshot props refresh operation admission independently of local draft identity. The callback checks instance policy, current full Scope/generation/token/revision, Ready/Saved, workspace, selected-key membership, effective layer and field compatibility; exact operation outcomes retain their reviewed authority. Children are placed in the existing selected-key Inspector. The compiler changes preserve logic, and aria-invalid follows the displayed field error. Styling is confined to binding classes.

Exact SHA-256 identities:

- presentation.rs: `19dfad60bc12ee64bcf2fbdc125b1ed1c8af628cdc94eca7a3c93efc2942b51f`
- keymap.rs: `64da4483797efbafc661aea6d8eb7e6309f5723d4701c36be287e943f9ea4955`
- panel.rs: `7b7274a143ab172b608e97af298cf3902be43ac143a561089595e4ae47d0b5bf`
- binding_controller.rs: `127d8305e68630a6c2dbde17e99ecfec13835b57164bbff77e9b16034a3a3d6b`
- binding_editor.rs: `3a32f90dc3b434fd9d5bbb76558c977aacacac7eeffcbe8b90232b3ddcccbf86`
- m1.css: `fd743eb98d48a13535d7891f3401155389c85240a99a34785867711f8445a9a4`

Final strict checks and paired public behavior/validation/recovery/history/reopen/compact interactions remain separate gates. Full F6K.2 and shared-selection acceptance are not implied. No source changes.
