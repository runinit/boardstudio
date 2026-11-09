# 03: Deduplicate the RendererHost page snapshot across host and runtime

Status: resolved
Type: build
Blocked by: None (can start immediately)

**What to build:** Eliminate the ~927-line `include!`-copied renderer host snapshot and its native source-equality guard in Runtime by giving the shared `host` renderer lifetime the narrow primitives `RendererPageHost` needs—specifically sequence-checked scene submission with boolean acceptance, display-state and handle updates, client-coordinate picking/plane projection, object focus, and resize—without exposing internal fields or re-enabling unchecked `update_scene` on the page path.

## Acceptance criteria

- [x] `RendererPageHost` in Runtime delegates to the canonical `host` renderer implementation rather than an `include!`-copied source snapshot, and the duplicate snapshot file and native text-equality sync test are deleted.
- [x] Sequence-checked scene submission continues to reject non-increasing sequence numbers, require a boolean acceptance result from `setScene`, schedule a frame only when the scene is accepted, and refuse operations when the renderer is disposed or its WebGL context is lost.
- [x] Unchecked `update_scene` is not exposed on `RendererPageHost` (and removed from `host` if unused across the workspace), and internal `RendererHost` state remains encapsulated.
- [x] Focused tests cover sequence-monotonicity enforcement, rejected-scene frame suppression, and disposed/context-lost guards, and `python3 scripts/check.py quick` passes.

## Implementation — awaiting parent verification

- Moved the existing checked `RendererPageHost` adapter and sequence stamping into the canonical host module; Runtime re-exports it. Keeping the adapter beside the lifetime avoids exposing fields or adding a parallel forwarding interface. Added its requested guarded resize operation; the existing lifecycle still owns automatic resize/DPR handling.
- Removed the snapshot, include-based extensions and native source-equality guard. Removed unused unchecked `RendererHost::update_scene` after a workspace caller search found no callers.
- Preserved all six canonical lifecycle test names. Their identical Runtime snapshot copies disappear with the snapshot, not their coverage. Removed only `renderer_host_source_sync::private_page_host_matches_normalized_library_host` from the native name baseline because source equality is intentionally no longer a contract.
- Added browser behavior tests for monotonic sequences (including rejected attempts), boolean acceptance and frame suppression, and every page operation's disposed/context-lost guard. Test setup injects the scripted renderer through the existing private attachment fixture; assertions exercise the public page adapter.
- No cargo, wasm-pack or check.py build/test commands were run by this subagent. Rustfmt and scoped `git diff --check` completed; the three mutation specs in `.evidence/cleanup03-mutations.json` were validated for exact edit matches only. No RED/GREEN result is claimed.

Parent verification (run sequentially, with the shared target exclusively owned):

```sh
wasm-pack test --headless --chrome web/crates/host --locked --features page --lib -- renderer_host::lifecycle_tests
python3 scripts/mutation-check.py .evidence/cleanup03-mutations.json --command "wasm-pack test --headless --chrome web/crates/host --locked --features page --lib -- renderer_host::lifecycle_tests" --evidence-dir .evidence/cleanup03-mutations
wasm-pack test --headless --chrome web/crates/host --locked --features page --lib -- renderer_host::lifecycle_tests
python3 scripts/check.py quick
```

The mutation run is intended to supply behavioral RED evidence for allowing equal sequences, scheduling rejected scenes, and bypassing the inactive guard. The focused commands before and after it establish baseline/restored GREEN. Public host exposure and cross-crate re-export require the parent's combined spec/standards review before closure. Leave this ticket claimed until checks and review are complete.

## Verification evidence

- Focused browser run: all 9 canonical renderer lifecycle tests passed; `.evidence/cleanup/renderer-tests.log`.
- Behavioral mutations: all 3 killed, none survived, no compile failures; `.evidence/cleanup/renderer-mutations.log`.
- Combined review: no blocking standards violations or spec findings. The small pre-existing duplicated JS-error formatter is deliberately outside this cleanup.

## Answer

Runtime now re-exports the checked page adapter from the canonical host; there is no renderer snapshot, equality guard, unchecked scene-update method, or public renderer state. Required quick acceptance passed with `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules BOARDSTUDIO_CHECK_JOBS=1`: 2,024 native tests, all crate browser launches (including all 9 canonical lifecycle tests after mutation restoration), and all 77 page tests passed. Receipt: `.evidence/cleanup/quick-retry.json.md`.

The full run passed build and provider/drag smoke checks but hit an isolated setup-guide launch timeout; the serialized quick retry passed that launch and completed every remaining browser/page check. Full-run receipt: `.evidence/cleanup/full.json.md`.
