# Pointer interaction range-capture diagnostic

Status: focused presentation candidate prepared and public selection behavior checked. This is not a final performance verdict or M1 acceptance. The focused page build uses the proposed one-file presentation change; unchanged providers are copied from the verified `8f509433` root stage. Offline setup is intentionally omitted from this component artifact.

## Red evidence and ranked diagnosis

The public pointer harness on the final `8f509433` candidate reports a second-session 100-key p95 of 134 ms against the existing 50 ms ceiling. The first completed candidate session records p95 22.9 ms at 30 keys (33 ms ceiling), 135.3 ms at 100 keys (50 ms ceiling), and 495.8 ms at 200 keys (100 ms ceiling). Those single-session samples support a public regression but are not the final paired-reference verdict or causal attribution.

The strongest source-level scaling candidate is `web/src/presentation.rs`: `visible_ids` contains the ordered visible part IDs, and the render loop clones that full `Vec<String>` once per visible SVG part to capture it for that part's pointer handler. The 30/100/200-key fixtures render 90/300/600 parts, so a signal rerender deep-clones approximately 8,100/90,000/360,000 ID strings in those per-part captures. `Runtime::submit` notifies the Dioxus tree after each submitted gesture sample, so the repeated render work is on the public pointer path.

Other costs remain hypotheses and are deliberately unchanged: the SVG loop linearly searches `scene.transforms` once per visible part; every pointer submission rebuilds the presentation; and each normalized single-part drag runs the existing board geometry-snap candidate scan. No measurement yet isolates their contributions.

The proposed change shares the same ordered `Vec<String>` through `Rc<Vec<String>>` captures. It clones that vector into an owned `Vec` only inside the existing Shift-Range branch, preserving the selection input order and avoiding copies for ordinary and Toggle clicks. No session, runtime notification, snap, transform lookup, public API or budget changed.

## Focused verification

- `cargo fmt --manifest-path web/Cargo.toml --check` — pass.
- `cargo test --manifest-path web/Cargo.toml --locked --no-default-features --features page,cad-worker` — 12 passed.
- `cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --all-targets --no-default-features --features page -- -D warnings` — pass.
- `dx build --web --release --base-path / --no-default-features --features page --cargo-args=--locked` — pass; the three page outputs are recorded with SHA-256 identities in [focused asset provenance](pointer-range-cache-assets.json).
- Public selection flow on the focused page at `http://127.0.0.1:46911/`, 1280×720, DPR 1: native `agent-browser click` selected `SW1` (Replace); Shift-modified DOM PointerEvent through the public SVG handler selected ordered `SW1`, diode 24, `SW2`, diode 23, `SW3`; Ctrl-modified DOM PointerEvents removed `SW2` then added `SW4` (Toggle); a native click selected only `SW5` (Replace). The focused build produced no browser console/page errors.

The modifier paths used DOM-dispatched `PointerEvent`s because the installed `agent-browser click` command does not expose modifier-click input. They exercised the rendered public pointer handler, but are not evidence of physical modifier delivery. The server omitted `service-worker.js` and its initializer intentionally; the expected service-worker 404 means this stage is not an offline or complete release artifact. A separate post-fix public pointer timing run and native-modifier QA remain necessary before attributing or accepting a performance improvement.

The browser session was closed after selection verification. Its server is left running at the URL above for the independently coordinated focused pointer measurement. See the asset provenance record for exact page/provider hashes and source identity.
