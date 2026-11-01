# 11: Gate wasm-bindgen in Core

Status: ready-for-agent
Type: build
Blocked by: [02](02-measurement-kit.md)
Parent: [map](../map.md)

## Question

Native builds compile `js-sys` and `wasm-bindgen` only for Core. Can those become wasm-only?

## Evidence

- `core/Cargo.toml` lists `wasm-bindgen` and `js-sys` as plain dependencies. `cargo tree --workspace -i
  js-sys` and `-i wasm-bindgen` show Core as the only native user in the root workspace.
- Ungated use: `core/src/lib.rs:42, 44, 56, 58, 100, 188-213`. The only `js_sys` use is `archive_request`.
- `CoreEngine` itself must stay native: 68 files in `core/tests` use it. Its methods take plain strings, so
  only the attributes need gating. `core/tests/support/boundary.rs` mentions the exports only in a comment.
- Cost: 23 CPU-s across 12 units in the root host tree, and 4.2 s of attributed wall. The brief's 30 CPU-s
  is close. The other 21.6 CPU-s in the CAD host tree comes from `cad/wasm`'s own dependency and tests, and
  is not affected.

## Gain

| Cold wall | CPU | Warm |
|---:|---:|---|
| about -4 s | -23 (-0.7%) | slightly less for Core edits, not measured |

Effort: 30 minutes. Risk: low. If a native test or tool turns out to need the export, the compile fails at
once. Low value, justified only by being this cheap.

## Work

- Move both dependencies to `[target.'cfg(target_arch = "wasm32")'.dependencies]` in `core/Cargo.toml`.
- Use `#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]` at `lib.rs:44, 56, 100`, the same for `constructor`
  at `:58`, gate the `use` at `:42`, and put `#[cfg(target_arch = "wasm32")]` on `archive_request` at `:188`.
- Regenerate `Cargo.lock` offline.

## Verification

```sh
cargo tree --locked --offline --workspace -i js-sys -e normal,dev 2>&1 | head -3   # no native path
cargo clippy -p boardstudio_core --all-targets --locked -- -D warnings
cargo clippy -p boardstudio_core --target wasm32-unknown-unknown --locked -- -D warnings
python3 scripts/check.py test build browser > .evidence/core-gating.log 2>&1
grep -E '^check: (native tests|all steps)' .evidence/core-gating.log
```

The wasm worker exports must stay identical. Compare `web/target/providers/core-worker/m1_core_worker.d.ts`
before and after; the diff must be empty.

## Stop rule

Revert if the measured saving is under 15 CPU-s, or if the `.d.ts` changes at all.
