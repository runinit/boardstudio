# F7 canonical Layout source and pick capability implementation receipt

**Reviewed contract:** Issue/draft `11-f7-canonical-layout-source-picks.md`, planning clearance report SHA `e27095bf1e25b77be4006ebd4a2ccc754751ae5be6bacf60e29ddc9fb2b77a8a`. The contract remains bounded to the callable same-crate canonical Layout producer, source owner, common viewer route and guarded current picks.

**Exact source base:** `8cfd6bb79e9e10b788e007fd428145b1e37095d1`.

**Implementation commits:** `c5800212` (source, model delivery, viewer mount, F3.6 view controls and child contract update), `c2a8f7b6` (cancel the active mirrored-pair draft fully when entering 3D), and `290a6c0d` (retire source work when the accepted viewer request disappears or changes), and `5e829af4` (box the private authored request and guard model-error settlement by exact owner/lease). Full current source head: `5e829af4b1b003c212133f13139389fc6fa3ec11`. The source worktree is isolated at `/home/chris/.local/share/boardstudio/worktrees/layout-toolbar-parity-20261002`.

## Capability delivered

- Runtime captures the accepted `ProjectDoc`, accepted scene identity, active complete `Scope` (including an optional retained instance), snapshot token, revision and generation. It selects the unique requested board and that board's accepted contours; the instance participates in freshness only and does not alter canonical geometry.
- Authored boards use the existing Core `PreparePreview` / `FinishPreview` and preview-generator pipeline; the large request is boxed only inside the private Layout request enum. Enabled imported `BoardReference` boards use only their accepted asset descriptor, verify stored bytes against the accepted SHA-256, and call the existing Core `PreviewBoard` operation. No public API, Core schema, renderer, Case projection or Case document route was added.
- Runtime owns pending/published/error lifecycle with a revocable per-source lease. The async pipeline, Core executor identity, model resolver, asset loading and model decoding recheck the live complete source owner. Stale work cannot publish after a board/scope/source replacement or viewer unmount. The mounted consumer retires a pending or published source immediately when its accepted request disappears or its scope/token/revision changes; this also revokes the lease and clears delivered Layout model rows. If async model delivery later fails, Runtime reports it only when the captured published preview still matches the exact current owner and lease, including against same-scope generation replacement.
- Model path ownership is reused from the existing canonical document path helper and existing `ModelDeliveryAdapter`, verified asset store, renderer decoders and mesh cache. Layout model rows carry the Layout lease and are passed through the same shared viewer; they do not depend on a Case preview. The shared-viewer source switch also handles canonical scene identity, renderer lifetime and selection events.
- Picks map an existing renderer model reference to a unique current part on the accepted selected board, then pass the guarded context through the existing `SelectionAdapter`. A retained physical instance is included in the current-owner check, never used to project Case offsets into Layout.
- The F3.6 view group is mounted in Layout: 2D, 3D assembly, and Footprints in React order. Footprints reuses existing state and is hidden only while in 3D, then restored. The 3D route replaces the command pill with the Layout / PCB assembly context label, hides the 2D canvas and layer overlay, and mounts the Layout source in the shared viewer. Entering 3D cancels the scoped drag, controller/matrix placement, mirrored-pair placement/draft and menu owner. Returning to 2D keeps the current session camera and document selection.

## Source and test evidence

Source-tree hashes at the exact source head:

| Source | SHA-256 |
| --- | --- |
| `web/src/case_preview.rs` | `b5a994f0497db0d190e0f1685c033816d5ed4a2b039b7027e9889a8a737bc0f8` |
| `web/src/main.rs` | `af62e1ba67f9b397a5be99da41f2658871583b419a6aec61e2ac5f681987d0bf` |
| `web/src/presentation.rs` | `7704a5228ebaa7ac6bafb0667fe5603b77ec221dcad3b4fa39a609f698fb8e74` |
| `web/src/presentation/case_viewer.rs` | `7a4b253ed2a7d6e119d1093bbb7065ea6aad0836811711ce3108c7d87c20670a` |
| `web/src/presentation/context_summary.rs` | `a6337ea51f3f6a364b69b8e246798640fddbee5a35e767ee8e5827886ad6216e` |
| `web/src/presentation/layout_viewer.rs` | `2a810129c739a4b9b654f216ee217830c167d0f9fd5024f0c4be80bc2b5e1116` |
| `web/src/presentation/layout_viewer_source.rs` | `f0b4e3e887ead3ed2f9908cc258f7a8a0dda82087f6fc9bc43ef11763b46dacd` |
| `web/src/presentation/layout_workspace.rs` | `ce43eb54853fab318b05ca50f9a6157d56511dcb7c66bf75a8968738ead59163` |
| `web/src/presentation/model_delivery.rs` | `4732d0e76268f0d5e5d99d01fc9d5aba74bd674702c752c93aa969334f9c0948` |
| `web/src/presentation/shared_viewer.rs` | `08cf9ced015e9f407f63aa3211b32436fc0679139405a675d6d0f5686c92d689` |
| `web/src/runtime.rs` | `cc46f6ff5dfb932b16564a2cc5be6700247938263919443e6d9c47a641ce4c24` |
| `web/assets/m1.css` | `4cf64bfc1804e0ebc06c8728ba709efae77b0de00af4760932617da3c9eed5ec` |

Checks passed:

- `cargo fmt --manifest-path web/Cargo.toml --check`
- `cargo check --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --bin boardstudio-web --features page`
- `cargo test --manifest-path web/Cargo.toml --bin boardstudio-web --features page layout_viewer_source::tests -- --nocapture` — 9 passed. In addition to source/pick/retirement coverage, a suspended-A/replaced-B settlement regression exercises the exact owner+lease predicate used by Runtime, and a focused imported Layout model smoke resolves a captured preview row through its accepted BoardReference model asset, SHA-verified bytes and existing `ModelDeliveryAdapter` to a decoded mesh row. The smoke does not execute the Core artifact producer or mount the shared viewer.
- `cargo test --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --bin boardstudio-web --features page --no-run` — browser-target test module compiles.
- `cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --bin boardstudio-web --features page --all-targets -- -D warnings`
- `git diff --check`

A pinned-React browser check used an isolated `agent-browser` session at `http://127.0.0.1:5173/`, source `5a472a9426e6e38993361da402cd4ec730feb369`, with the Sofle v2 demo. Starting a Mirrored pair placement then clicking 3D removed the “Place linked halves” active-placement state; React showed its Layout / PCB assembly label and kept the 2D/3D group. Enabling Footprints, entering 3D, and returning to 2D showed Footprints absent in 3D and still pressed on return. This verifies the reference transition semantics; it is not a paired Dioxus build receipt.

A concrete implementation issue found and fixed: the initial shared-viewer route qualified model rows only through an optional Case preview, so a canonical Layout scene could never receive its own asynchronously delivered meshes. The route now qualifies Case rows against the Case preview and Layout rows against the exact current Layout snapshot/lease. This was found by source-path inspection before integration; no model-count or public-rendering claim is inferred from it.

## Remaining gates

- The integrated Dioxus candidate still needs the paired browser journey for the F3.6 view-group/scene transitions, 2D camera return, Footprints restoration, source-owner changes and real picked-part selection. The focused model smoke does not verify Core artifact execution or a mounted renderer consumer.
- Existing F7.3b issue 03 remains open for full shared-viewer consumer acceptance, including its BoardReference pose/elevation behavior, generated-keycap suppression, conditional same-document Case overlays and remaining lifecycle/consumer checks.
- Existing F3.6 camera/fit acceptance and F3.7 integrated Layout parity remain open. The pointer transform-tool gap and other child-owned command parity gaps are not closed by this view/source capability.
