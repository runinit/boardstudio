# Active outline reopen control correction — 2026-10-03

Source fix `e3ea603a8078b661993762e62207ed52f7a58390`, base `19525578207ff66bfb51a6c7a6483a0bb7905029`, isolated branch `codex/layout-outline-reopen-fix-20261003`. The change adds explicit selected flags to Generated and the accepted matching version option. Existing select value/change callbacks, operation guards, settings and persistence remain unchanged. No load/schema/backend change is necessary.

The existing actual production RED is the paired journey at source19525578/port34742, pinned React5a472a94, retained original screenshots under `/home/chris/.local/share/boardstudio/retained-tmp/20261003/layout-outline-journey/`. Additional read-only inspection of that same retained session is in `retained-browser-red.json`: stored revision12 has active outline-version-18/QA Outline and Chamfer2. Tree says QA Outline Active; Inspector fixed-description/name confirms accepted projection is fixed; native Active outline select incorrectly says Generated. This is a dynamic native-option mount mismatch, **not demonstrated data or settings loss**. The corrected journey receipt retains those distinctions; original receipt bytes are preserved separately (SHA07897f0fc0b92ea74c0d4de11354a7198126339f01cdc2d0446369f36f334e1e) and original screenshots are unchanged.

One focused mounted regression uses the existing production outline lifecycle and OutlineVersionInspector with an already accepted fixed-version snapshot. It asserts the initial native selected value, independent saved Chamfer7.25, and no submitted edit. This controlled accepted-state mount is not a full BrowserStore/Session reopen or Editor/history journey. It targets the diagnosed rendering boundary; actual public saved/accepted data already establishes that load is not resetting the version.

The pre-fix test run fails at the exact selector assertion: `left Some("")`, expected `Some("saved-outline")`. The same test with the two flags passes **1/1**, 0 ignored (165 other tests filtered). Only this regression was run. Command:

```sh
CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/f73b-layout-layer-controls-20261002/web/target \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/chris/.cache/.wasm-pack/wasm-bindgen-9411bdb1a3e2bbb9/wasm-bindgen-test-runner \
WASM_BINDGEN_TEST_ONLY_WEB=1 WASM_BINDGEN_USE_BROWSER=1 WASM_BINDGEN_USE_HEADLESS=1 \
CHROMEDRIVER=/usr/bin/chromedriver \
cargo test --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --locked \
  --no-default-features --features page --bin boardstudio-web \
  mounted_reopened_fixed_outline_selects_its_saved_version -- --nocapture
```

Root explicitly assigned this non-root cached target lease; it is released after the green run. Raw stdout/stderr bytes are retained as deterministic `focused-{red,green}.log.gz`; readable `.log` copies remove only trailing whitespace. Raw uncompressed hashes are in `audit.json`. Source/test rustfmt check and diff-check passed. The page-WASM test compiled the affected code; no extra broad suite/build/strict Clippy was run in this isolate. Root owns its combined affected qualification and next packaged candidate.

Public corrected-candidate reopen is the remaining focused verification. Perimeter editor, paired edit/Undo/Redo/save-reopen and full F3.4/F3.7 criteria remain open; this repair does not close parents. No new refactoring takeaway beyond existing RF-009's need for real native DOM/public widget proof was observed. No other author or root source was edited.
