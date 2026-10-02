# Parts catalogue native harness — compile blocked

Harness inputs under this directory are read-only snapshots from the integrated worktree. The copied module is included at the original private path `crate::presentation::parts::catalogue`; its original `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../app/src/parts/imported-parts.json"))` resolves through `harness/../app/src/parts/imported-parts.json`, copied byte-for-byte from the source data.

## Provenance

- Worktree HEAD at inspection: `515f390daa412d38a7d905b101df6796a62f8ca1`.
- `web/src/presentation/parts/catalogue.rs` and frozen `catalogue.rs.source-515f390` SHA-256: `6029240c43c292503eb4deed7c576bcadc3404e4a91637c007d90b48fc0bd736`.
- `app/src/parts/imported-parts.json` and temp copy SHA-256: `000f4ba13114305c33e1378806c25840d903fa335da559b88c9d9404731acd7f`.
- The real module contains 7 embedded `#[test]` functions. The static import decode test expects 9 definitions; this is 7 tests over 9 imported entries, not 9 tests.

## Compile attempt

Command:

```text
CARGO_TARGET_DIR=/home/chris/.cache/boardstudio/parts-contract-tests cargo test --manifest-path /tmp/frontend-run/parts-contract-tests/harness/Cargo.toml -- --nocapture
```

Compilation stopped before the test runner with E0277 at copied `catalogue.rs:94`: `format!("{:x}", Sha256::digest(...))` requires `LowerHex` for sha2 0.11.0's `hybrid_array::Array<u8, U32>`, which does not implement that trait. The harness uses the same exact sha2 version and disabled defaults as `web/Cargo.toml`. I preserved the exact production source and did not shim or edit its hashing behavior, so none of the 7 tests ran.

The harness includes a clearly labeled fail-closed `runtime::resource_url` stub solely to satisfy the reference from this mixed source module. The loader is never called. This does not exercise browser URL resolution, dynamic import, JS providers, cache behavior, offline behavior, or Session integration.

## Build target storage

The initial native build target ran out of space on the 16G `/tmp` tmpfs. It was moved intact to `/home/chris/.cache/boardstudio/parts-contract-tests-failed-target/target`. The retry target is `/home/chris/.cache/boardstudio/parts-contract-tests`. Test source, module snapshot, copied data and this evidence remain under `/tmp/frontend-run/parts-contract-tests`.

## Corrected-source rerun

The exact frozen module snapshot `catalogue.rs.source-4b05d451` was copied from worktree HEAD `4b05d4515bbcbfc8df666778a8171bb735a59c70` in `/home/chris/.local/share/boardstudio/worktrees/parts-f41a-20261002`. Its SHA-256 is `1ba3b7088ac9d613812cd389e4bc953d97b42a1c4dffb7e01cd939613900cb46`. The imported JSON fixture and its copy still match at SHA-256 `000f4ba13114305c33e1378806c25840d903fa335da559b88c9d9404731acd7f`.

The corrected source formats digest bytes individually as lowercase hex and includes the expected imported JSON digest test. This exact source and data passed all **8 embedded tests**:

```text
CARGO_TARGET_DIR=/home/chris/.cache/boardstudio/parts-contract-tests cargo test --manifest-path /tmp/frontend-run/parts-contract-tests/harness/Cargo.toml -- --nocapture

running 8 tests
... 8 passed; 0 failed
```

The eight are the seven original pure tests plus `imported_source_hash_matches_the_bundled_source_digest`. The decode test asserts the nine imported static definitions all deserialize as the existing Core `PartDefinition` type. Merge precedence/order, `Rc` identity, category/search alias behavior, JSON parameter serialization, regex line terminators, and catalogue filtering all ran from the copied production module.

The native harness emitted dead-code warnings because it includes the whole production module while invoking only its embedded pure tests. It did not call `load_bundled` or the URL stub, so this remains native pure-logic evidence only; no JS import/provider, browser URL, cache or offline behavior was exercised. The test target is in `/home/chris/.cache/boardstudio/parts-contract-tests`; all harness source/data/evidence remain in this directory.
