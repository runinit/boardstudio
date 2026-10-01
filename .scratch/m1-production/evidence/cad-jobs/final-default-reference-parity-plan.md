# Same-input final REVIUNG mechanical parity

## Result

Completed against maintained release `m1-release-20261001-8f509433` (source commit `8f509433bacb84935dc1e1bbbb3840cd56b33029`). The test used the actual default-config REVIUNG editor to export the archive, then unpacked that archive with the existing TypeScript `unpackProject` and exported it through the existing `exportMechanical` service and public Core/Case/Export clients. No Rust mapper was duplicated. The QA origin (`http://127.0.0.1:46882`) was isolated from release storage; its Vite server and browser sessions were closed after the run.

The captured archive is `web/target/builds/m1-release-20261001-8f509433/qa-reviung41-default-public-export.boardstudio`, 1,397,533 bytes, SHA-256 `7acb9d0d7f52d00359f4c8cce0e4a4c2df368ff524949c90651836d94338f54b`. It contains project `edbec892-9952-4b6b-a7ba-cb6b0a514ac4`, revision 4, and no physical instance ID. The reference UI initializes selection to the empty ID, so the canonical no-instance scope was used. Mechanical settings were the captured document defaults (`printed`, `tray`, 1.6 mm PCB); the shared construction's gasket setting was not selected by this UI scope. The complete config and source file hashes are retained in `reference-parity-default-input.json`.

The existing TypeScript reference produced a five-body assembly containing the nominal unpopulated PCB reference. Its STEP was 13,662,809 bytes, SHA-256 `a8b7fb8288eeed2e29c06ebcfaa290767625879e175579aa9dd61f5c190d0cda`. The final M1 UI STEP is 13,662,781 bytes, SHA-256 `f946d69c2ad39abd3f799ca996067d7f89dba7cc361a9516251c9e32307722f4`. Exact-byte parity therefore fails. Both files have 303,613 lines; 434 records differ, all `DIRECTION` or `CARTESIAN_POINT` records. Numeric fields differ by at most `1.000444171950221e-11`; the first example is a direction coefficient serialized as `0.707106781186` vs `0.707106781187`. Do not characterize this as a header-only difference.

Geometry agrees at the measured reader precision. Both public meshes have 41,736 vertices, absolute signed volume `217359.0776248333 mm³`, and matching bounds (`[-1.5260000228881836,-90.87899780273438,-6.599999904632568]` to `[288.45001220703125,17.09000015258789,5]`). Independent OCCT BRep readback finds five solids. Reference STEP volume is `217359.0807798726 mm³`; M1 STEP volume is `217359.08077987254 mm³`, a delta of `5.820766091346741e-11 mm³`; bounds agree within `5.7e-14 mm` on the maximum X value. This supports geometry parity, not identical serialization.

## Evidence and reproduction

- `reference-parity-default-input.json`: input archive, project/default scope, mechanics config, and source hashes.
- `reference-parity-default-step.json`: full actual public TypeScript reference run, requests, outputs, and returned geometry.
- `readback-reference-reviung41.json`: unchanged-source public M1 `readStepModel` and independent STEP parser comparison.
- `reference-parity-step-text-diff.json`: byte hashes, differing records, numeric deltas, and interpretation.
- `reference-parity-default-step-initial.json`: initial run record.
- `final-default-reference-parity.html` and `.ts`: isolated harness invoking existing application services.
- `inspect-reference-reviung-step.mjs`: parser/readback evidence generator.
- `web/target/builds/m1-release-20261001-8f509433/qa-reference-reviung41-default.step`: retained reference STEP bytes; `qa-reviung41.step` is the paired M1 download.

The `reference-parity-main-selected-diagnostic.json` records an earlier exploratory harness mistake: it selected instance `main` although the public default has no selected instance. That exercised a different 58-body gasket configuration and failed later in screw association packaging. It is excluded from the default-scope result and is not a default export regression.

Exact reproduction should use the captured archive and harness with the same source files/hashes above, an isolated loopback origin, and the canonical empty selected-instance scope. Do not use the historical explicit-main diagnostic as the reference case.
