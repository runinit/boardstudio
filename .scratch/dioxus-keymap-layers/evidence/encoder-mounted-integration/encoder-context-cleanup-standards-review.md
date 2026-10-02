# Encoder context cleanup — Standards review

Reviewed the exact working-tree delta against `fa355edd` in `frontend-keymap-layers-20261002`, limited to `web/src/presentation/keymap/binding_controller.rs`. SHA-256: `e5e96a4b35e2b7590d8072b08822fec0e2a8f17440bcb810f67dfb9cb6c36898`.

No material Standards findings. `BindingProjectionSources` groups the existing private hook inputs without widening the library API or introducing a second source of state. Its `Rc` and memo handles preserve ownership and do not copy the underlying Keymap or document. `BindingReadContext` borrows the same accepted snapshot, projections, and choices for the duration of each synchronous read; the local live-input binding makes that borrow explicit.

`EncoderIdentityCheck` replaces the previous boolean without changing its mapping: admission uses exact identity, while terminal settlement and feedback use stable input lineage. All read sites preserve their prior policy and live-getter usage. Borrowing the reactive layer ID addresses the owned reactive tuple without changing the selected layer. Pending operation release, field merging, memo dependencies, and callback guards are unchanged.

The grouped contexts and named identity policy improve clarity under the repository’s ownership and narrow-interface conventions. No additional refactoring is warranted for this bounded delta. Source review only: no Cargo or tests run. The root caller must use the grouped private input type; compiler and public encoder acceptance remain separate gates.
