# Native Keymap layer Session/Core harness (prepared, not run)

## Provenance

- Target integration checkout: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`
- Checkout HEAD when prepared: `566e180fcffab6ba2a78535a60c1089c2ac0a279` (`Integrate independently reviewed private shared viewer source`).
- Harness is isolated at `/tmp/frontend-run/keymap-layer-native`; its Cargo dependencies point directly at this checkout's `application/` and `core/` crates; the Application dependency is explicitly aliased from package `boardstudio-application` to Rust crate name `boardstudio_application`. No Cargo command, compiler, or test command was run; parent owns the compiler turn.
- It uses real `CoreEngine` and `Session` requests/completions and the existing `Session` effects/terminal outcomes. The fixture is an actual `ProjectDoc` opened and accepted by Core; it does not construct `AcceptedSnapshot`, `SceneDelta`, or a pretend ReadModel.

## Coverage prepared

`src/lib.rs` contains four focused tests, with all harness helpers and tests gated under `cfg(test)` to avoid library-target dead-code warnings:

1. Opening a document with `keymap: None`, then applying Core's actual `KeymapChange::AddLayer` through `Event::Edit`, proves virtual Base materializes in saved order before the new layer; checks `Completed`, saved revision, and the exact resulting IDs/names.
2. Renaming through Session/Core preserves the target layer ID; an actual `Event::Undo` restores the prior saved name/ID/data while advancing the saved revision to 3 (Undo itself is a new committed Core operation). Checks exact terminal outcomes.
3. Removing a non-base layer through Session/Core then undoing restores that layer's stable ID and name. Checks exact terminal outcomes.
4. Removing Base is rejected with the Core/Session terminal reason and leaves the accepted document and revision unchanged.

## Explicit limits

This does not prove the private Dioxus controller's retained requested ID, fallback display, feedback admission, or WASM callback lifecycle. The integrated checkout has no `web/src/presentation/keymap/layer_controller.rs` yet; the candidate controller is still being corrected in the author's separate worktree and depends on Dioxus signals plus `crate::runtime::Runtime`, which the native target does not compile. I intentionally did not recreate its selection state in this harness or rely on its hand-built unit fixtures. Root can review/run these Core/Session checks now, then decide whether a later pure production seam or public WASM test is needed for controller retention/fallback.

## Source hashes

```
a2a0764e90efeacf74482f7a399350914e9f049511af6b4e0478ec1e9ba5538b  core/src/keymap.rs
e2ea75daa133195a25a877ef33e03e9a971f49ead3d70ef10a8f1d51dcb2efea  core/src/model.rs
403c5eb0a375bb3d2e9456e6605d6d04573d5a9d1908f1eef523cc4f03f93fb3  application/src/session.rs
57ada5808f38fdd7cb9bbf834069cbb5d91c6c85fdf4318b2c39278ef75dbc3b  application/tests/durable_session.rs
```
