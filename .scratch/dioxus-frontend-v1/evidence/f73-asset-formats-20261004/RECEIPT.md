# F7.3-C06 STL/WRL Parts qualification — 2026-10-04

## Scope and source

This qualifies real STL and WRL bytes through the public Parts model import, durable project asset storage, format decoder, and mounted sample scene on candidate `4e716156` at `http://127.0.0.1:34822/`. Runtime source comparison to current HEAD `e2ec1c1e` found only a test-module `cfg` change in `web/src/presentation/keycaps_fit.rs`; the relevant runtime path remains unchanged.

Relevant seams: `web/src/presentation/parts/assembly_editor.rs` exposes a file input accepting `.step,.stp,.stl,.wrl`; `web/src/presentation/model_asset_import.rs` derives the format/media type, hashes and verifies bytes, then persists them through `BrowserStore`; `web/src/presentation/model_delivery.rs` dispatches STL and WRL bytes to their respective decoder ports; `web/src/presentation/parts/preview.rs::PartsSampleViewer` mounts the isolated Parts preview through `CaseSharedViewer`, validating owner, scope and snapshot token for events.

## Preserved fixture bytes

| Format | Original repository path | Bytes | SHA-256 |
|---|---|---:|---|
| WRL / VRML | `ergogen/library/vendor/koktoh/3d_models/Choc_V2_Red.wrl` | 2,855,526 | `99c84a3b99fe565ba7de2cced7b6a3b235cb48a0460b5e44ffad0745c78d0972` |
| Binary STL | `ergogen/library/vendor/kiswitch/3d_models/SW_Hotswap_Kailh_Choc_V1.stp.stl` | 292,484 | `cf007a5ced60b58f0b7a94b842accb32555f3e63e33364e455fea786c0fcf59f` |

The browser chooser used those tracked repository paths directly. Temporary byte-for-byte receipt copies were hash-checked and removed to avoid storing 3.1 MB of duplicate assets; Git retains the exact source bytes. The STL length matches its binary triangle-count framing (`84 + 50 × 5,848`).

## Public interaction result

The recipe `Archived STL WRL qualification20261004` was created in the isolated `PCB grouped qualification20261004` fixture. The operator explicitly selected component `ergogen:ceoloide/switch_mx` before importing, since changing the component clears the model list. WRL was imported first and STL second, then the assembly was saved. Following full reload and reopen, the assembly retained both models and reported `3D preview ready`:

- Model 1: `Choc_V2_Red.wrl`, asset reference `model-asset-25`, SHA matching the WRL fixture above.
- Model 2: `SW_Hotswap_Kailh_Choc_V1.stp.stl`, asset reference `model-asset-22`, SHA matching the STL fixture above.
- Assembly scene rows: `Hide P1 · 99c84a3b99fe565ba7de2cced7b6a3b235cb48a0460b5e44ffad0745c78d0972.wrl` and `Hide P1 · cf007a5ced60b58f0b7a94b842accb32555f3e63e33364e455fea786c0fcf59f.stl`.

Each row was hidden and restored independently while the other remained pressed. Both ended restored; no alert appeared. Project menu showed `PCB grouped qualification20261004`, `Revision 10 · Saved`, and `Saved in this browser`. This is provider/decoder/scene persistence evidence and independently actionable per-model visibility evidence. It does not claim a portable archive export/import or a Parts sample-pick result.

## Reference limitation

Before reload, the paired reference imported and saved both models and reported `2 / 2 models · 1.6 mm PCB`. The paired reference page at `127.0.0.1:5175` failed reload with `net::ERR_CONNECTION_REFUSED`. No live listener or attributable server launch/build provenance was available. The pinned TypeScript source and two distinct `app/dist` directories do not identify the historical served output, so no guessed restart or redirection was performed. Reference-side reload comparison remains unavailable; this is not a product failure result.

## Candidate 3D theme continuation

Using the same saved two-model recipe on the same served source, public Project → Workspace settings → Color theme selected Light then Dark while the real Parts sample canvas was mounted. Light reported `3D preview ready`; Dark settled to `3D preview updated`. Both had no alert, retained the actual isolated read-only Parts canvas (368 × 183 backing attributes at this desktop layout), both Model1/Model2 editors and the recipe name. Dark-theme 3D→2D→3D returned to `3D preview ready`. Project menu remained Revision10 · Saved. Restored System theme. This supplements F7.3-C07's theme/form continuity branch only; no DPR/resize/context-loss, screenshot/pixel-color or reference-theme claim is made.

## Recovered pinned reference continuation

An isolated fresh exact-pin build restored port5175; see `../reference-recovery-20261004.md` for source/build/server provenance. Browser script `assets/main-CYxchQWA.js` matched the rebuilt index. The retained `PCB grouped qualification20261004` project reopened, and `Archived STL WRL qualification20261004` retained both saved models after reload: WRL asset UUID `f01fbbf9-052a-46fc-af6c-7e650ae4f25c` and STL UUID `44497083-8d0e-45f2-aaad-6f24209d0376`, with their original filenames. This completes the interrupted reference persistence observation.

The recovered reference's renderer reported `0 / 2 models · 1.6 mm PCB` and the real alert `WebGL2 could not start. The 2D editor remains available.` No successful post-recovery rendering is claimed; the earlier paired2/2 observation and the current candidate's successful reload/render remain separately attributed. The cause of this new reference initialization failure has not been established, and pinned reference source was not changed.
