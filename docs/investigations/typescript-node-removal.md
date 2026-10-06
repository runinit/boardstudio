# TypeScript and Node removal assessment

Assessed: 2026-10-05. Inspected revision: `3cdeb2ac2`.
Status: implemented. This document retains its original inventory and records how
each item was resolved. Items 1–3 landed with the [Rust footprint cutover](footprint-generators-rust.md);
items 4–10 removed the CAD adapters, generated contracts, Node tests and checks,
maintenance commands, package infrastructure and CI steps. Only the remaining
generated wasm-bindgen glue and vendored sources are JavaScript.

The Dioxus UI is already in Rust. Remaining TypeScript and Node dependencies
provide footprint generation, PCB worker behavior, bundled content preparation,
CAD build orchestration, tests, maintenance commands and CI.

This was a read-only source, import, manifest and workflow assessment. No build or
test suites were run. Recheck consumers and commands before implementing a slice.
File links below identify the assessed implementation, not permanent interfaces.

## Scope

The objective is to remove TypeScript and the project's requirement for Node,
npm and pnpm while preserving current functionality and meaningful checks.
Python tooling and Rust are already established in this repository.

Removing TypeScript and Node does not require removing browser JavaScript.
Generated WASM bindings and trusted JavaScript footprint generators can remain.
Moving all handwritten generator logic into Rust would be an additional scope
decision. The proposed sequence retains those generator bodies initially.

Existing user files and browser data must be preserved. Old-client compatibility
and the retired React frontend are not part of this work.

## Inventory

At the inspected revision, tracked files included:

- 273 TypeScript files: 242 generated contract files and 31 other files.
- 96 JavaScript files, including generated output, providers, scripts and tests.
- 36 bundled footprint generators: 24 from the main library and 12 selected
  from the vendored infused-kim library. Three additional vendored JavaScript
  generators are excluded by the catalogue builder.

Counts describe source inventory, not effort. Generated contracts are largely
removal work; provider behavior and independent validation require replacement.

| Item | Remaining dependency and required change | Relative effort |
| --- | --- | --- |
| 1. Footprint provider | Replace TypeScript parameter handling, geometry extraction, terminals, model bindings and rendering adaptation in `ergogen/src/index.ts`. Port all generator bodies to Rust and delete the JavaScript; see [the footprint generator plan](footprint-generators-rust.md). | High |
| 2. PCB generation worker | Move job validation, net allocation, KiCad form conversion, arc upgrades and model-path rewriting from `preview-generator-worker.ts` and `kicad/src/ergogen.ts` into Rust. Rendering moves into Core and the worker is removed; see [the footprint generator plan](footprint-generators-rust.md). | Medium–high |
| 3. Bundled project preparation | Replace 14 TypeScript files in `tooling/demo-projects` and `prepare-demo-projects.mjs` with a native Rust content builder using core document/archive APIs. Preserve bundled examples, embedded models, hashes and provenance. This removes the active Vite dependency. | Medium–high |
| 4. CAD build orchestration | Implemented in Python in [cad/scripts](../../cad/scripts). Pinned OCCT downloads, checksum verification, native setup and container-based WASM builds are retained. The web provider builder calls Python directly. | Complete |
| 5. CAD adapters and validation | Complete. The TypeScript adapters, Node CAD tests and the libcascade dependency are removed. Their assertions run as native Rust tests ([validation tests](../../cad/wasm/src/model/construction/validation_tests.rs), [source tests](../../cad/wasm/src/model/source_validation_tests.rs)) against kernel-agnostic [expectations](../../cad/test/fixtures/step-expectations.json) and a native [OCCT STEP oracle](../../cad/step-oracle) (adapter-independent, not kernel-independent). TS-only preview and metrics facades were retired without replacement because nothing consumed them. | Complete |
| 6. TypeScript contracts | Complete. The generated TypeScript, the handwritten facade, the Node generation and runtime-import checks, `ts-rs`, the `export-types` features and the TS-specific Rust annotations are removed. [Shared Rust contracts](../../contracts/rust) and serialization checks in `core/tests/contracts.rs` remain. | Complete |
| 7. Node tests and checks | Complete. KiCad integration checks are native Rust ([tests](../../core/tests/kicad_integration.rs)). Native/WASM boundary parity replays shared requests against a golden transcript natively ([test](../../core/tests/boundary_parity.rs)) and through the real WASM exports in headless Chrome ([test](../../web/src/boundary_parity.rs)). The repository check is [check-doc-links.py](../../scripts/check-doc-links.py); the TypeScript module-graph analysis was retired with the last TypeScript module. | Complete |
| 8. Content maintenance commands | Complete. The KiCad-part and VIK-module importers and the keyboard and Sofle layout extractors are Python ([scripts](../../scripts)), sharing a KiCad form reader and a JSON writer that reproduces JavaScript's output exactly. Both importers regenerate their committed snapshots byte for byte; the extractors were checked against the removed implementation on shared inputs but not against upstream checkouts. | Complete |
| 9. Development and build entrypoints | Complete. [dev-web.py](../../scripts/dev-web.py), [build-web.py](../../scripts/build-web.py), [serve-web.py](../../scripts/serve-web.py) and [run-wasm-tests.py](../../scripts/run-wasm-tests.py) call no Node tool. [check.py](../../scripts/check.py) replaces the composite package scripts (`check`, `test`, `typecheck`, `test:browser`, `check:security`, `precommit`). | Complete |
| 10. CI, manifests and documentation | Complete. The workflows no longer install Node or pnpm or run a Node-version matrix, and Dependabot no longer tracks npm. `package.json`, the pnpm lockfile and workspace, `.node-version` and `.npmrc` are removed, and the development instructions use the Python entrypoints. | Complete |

## Proposed sequence

1. **Establish a behavior baseline.** Capture representative generator outputs,
   PCB exports, bundled archives, CAD validation results and worker behavior while
   the existing implementations remain available. Map tests to replacement checks.
2. **Replace build orchestration and maintenance tooling.** Port CAD orchestration
   and suitable maintenance/check scripts to Python. Introduce direct development,
   build and check commands; keep unresolved Node-backed steps until replaced.
3. **Replace bundled-project preparation.** Reuse native Rust core APIs. Verify
   equivalent examples, configuration and embedded assets before removing Vite.
4. **Migrate runtime providers.** Move TypeScript adapter and PCB-worker behavior
   into Rust. Package retained JavaScript generator bodies without Node. Update
   browser consumers, asset staging and offline inventories together.
5. **Complete test migration and retire contracts.** Replace Node-only validation,
   including the independent CAD oracle. Remove TS adapters, generated contracts
   and Rust TS-export machinery only after their consumers have been retired.
6. **Remove package infrastructure and verify independence.** Delete remaining
   Node/TypeScript configuration and switch CI/documented commands. Run the entire
   workflow in a fresh environment without Node, npm, pnpm or cached generated
   assets.

The provider changes are the main product risk. Preserve parameter semantics,
net identity, generated geometry, model placement, worker ownership and export
behavior. Replacing a test runner must not silently remove its assertions, and
using the same CAD implementation for export and reimport must not silently
replace the existing independent oracle.

## Completion criteria

- [ ] No maintained TypeScript source or TypeScript compiler dependency remains.
- [ ] A fresh checkout can prepare content, develop, build, test, audit and package
  both root and `/boardstudio/` deployments without project-controlled Node/npm/
  pnpm execution or pre-existing generated assets.
- [ ] All retained content import, refresh and verification commands run without
  Node, including commands outside the default build/check path.
- [ ] Meaningful native, WASM, browser, provider, parity and independent CAD
  validation coverage has a working replacement.
- [ ] Browser checks cover generator edits, PCB/CAD exports, fresh-project
  save/reopen, bundled examples, model assets and offline behavior.
- [ ] Current development, CI, security and onboarding instructions describe the
  replacement workflow.
- [ ] Remaining JavaScript is explicitly accounted for: generated browser/WASM
  bindings and, if retained, trusted footprint generators and their browser glue.
- [ ] If zero TypeScript files includes build output, generated WASM `.d.ts`
  declarations are suppressed or removed as part of packaging.

The environment check concerns project toolchain requirements and commands.
Hosting-platform internals, such as the implementation runtime of a GitHub
Action, are a separate boundary from requiring Node to build this project.
