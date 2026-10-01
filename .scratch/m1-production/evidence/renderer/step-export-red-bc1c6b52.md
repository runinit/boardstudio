# STEP export red reproduction (diagnostic candidate only)

This is the original failed public-UI reproduction against build `m1-release-20261001-bc1c6b52`, source commit `bc1c6b524ec5a6350ca432472bf4ff543482aeb5`, served at `http://127.0.0.1:46656/`. The page and providers rendered Sofle v2 exact case geometry successfully, but this build stage retained multiple prior Dioxus page WASM outputs in its root public tree. It is preserved only as a diagnostic red repro, not valid release acceptance; reproduce against the clean-source stage before relying on it.

Commands/actions:

1. `agent-browser --session m1finalbc1c6b52-7dd31abc1fc4 find role button click --name 'Sofle v2 copy'`
2. `agent-browser --session m1finalbc1c6b52-7dd31abc1fc4 find role button click --name 'Generate case'`
3. Wait for “Exact case geometry ready.”, then click the interactive `Export STEP` button (the snapshot ref was `@e21`).
4. `agent-browser --session m1finalbc1c6b52-7dd31abc1fc4 download @e21 web/target/builds/m1-release-20261001-bc1c6b52/qa-sofle-left.step`
5. Inspect UI status, browser console, and instrumented `URL.createObjectURL`/`URL.revokeObjectURL` calls.

Observed red result: no STEP file was created; ordinary button click and `download` both returned without a download. The UI stayed at “Exact case geometry ready.” and “Saved locally.”, with no export completion/failure announcement. Instrumented blob URL lifecycle was empty (`[]`): no create and no revoke occurred. Browser console emitted `wasm-bindgen: imported JS function that was not marked as `catch` threw an error: undefined is not iterable (cannot read property Symbol(Symbol.iterator))`; the generated JS stack points to helper `__wbg_from_296ca31f8d0f1c52`, which returns `Array.from(t)`, followed by the page WASM. This matches an absent iterable argument while decoding the STEP-only CAD worker result; the raw worker message was not captured in this original run and must be captured on the clean candidate before making that payload claim.

The fresh-source reproduction should capture only the message metadata, result field names, bodies field presence/type, STEP byte length and leading bytes; do not retain the full potentially large payload. A corrected UI reproduction must show a downloaded `keyboard.step`-format file and one object URL created and then revoked after delivery.
