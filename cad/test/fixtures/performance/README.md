# Frozen 011 requests

These ordinary JSON inputs freeze the **literal DTOs** used by the historical
untimed admissions. No provider-dependent geometry conversion occurs in a timer.
The historical input/result/QA evidence remains read-only under
`/home/chris/.t3/worktrees/ts-boardstudio2/t3-2088b1aa/.evidence/011-historical-baseline/`.
Use its `c1/REPORT.md`, `c2/REPORT.md`, `v2/REPORT.md`, `*/admission-receipt.json`
and independent results for their original scopes and bindings. Those old
Monstertruck bindings are not acceptance of new post-007 artifacts.

| ID | Frozen input origin | Independent admission gates to retain |
| --- | --- | --- |
| C1plate | `c1/step-request.json`, from `cad/test/fixtures/holed-plate.json` | 120×80×2, 24 holes, 14400 mm³; topology, 126 material probes and independent STEP |
| C2trayboss | `c2/step-request.json`, historical Core `prepare_case` output for `cad/test/fixtures/boss-tray.json` | 12 bosses/bores, six windows, `28080 + 456π` mm³; 114 probes and analytic cylindrical STEP supports |
| C3gasket | `c3/step-request.json`, wrapping `cad/monstertruck/tests/fixtures/layered/gasketed-pair.prepared.json` | Two bodies, 9122 mm³ each; cavity, groove, inner/outer lip and planar STEP |
| P1analyticPCB | `pcb/step-request.json`, wrapping `cad/monstertruck/tests/fixtures/layered/pcb-simple.json` | 20×16×1.6, radius-1 drill, `(320−π)×1.6` mm³; cylindrical STEP, chord-support preview controls |
| K1keycap | `k1/request.json`, expanded literal spec from `cad/scripts/browser-smoke/smoke.html` | 25 DSA/MX caps, spherical 0.65 dish, O legend, 19.05 pitch, 15° local rotation; all placement/dish/socket/material checks |

C1/C2 requests preserve the historical Core preparation; C3/P1 preserve their
committed prepared fixtures. K1 intentionally preserves the retained request's
floating-point coordinate literals. Appearance admission does not admit fused
printing: `K1keycap:print` uses the same specs through `print-keycaps` and needs
separate closure, connected-material, socket/legend and binary STL admission on
both backends. Do not assume historical print semantics match just because its
operation name exists.

V1switch is the original
`ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step` (155 faces).
V2vendor is the `imported-multi-solid` entry in `cad/test/fixtures/manifest.json`:
`ergogen/library/vendor/infused-kim/3d_models/trackpoint/TP_Red_T460S_platform_z_offset_+0.0_pcb_offset_-2.0.step`
(63 solids/529 faces). Read the original files in place. Verify manifest SHA,
complete source/occurrence census, placements, line sets and face witnesses.
V1's previous refusal is historical; fresh current admission must retain all155
faces with default budget. A failed/default-refused row is never a speedup.

Qvendor uses the **existing authoritative** generated-analysis contract fixture's
qualification-v2 `directVendorRecipe` and `queryExamples`, real owned worker
references, original0805 bytes, and default/0.01 requests. No report or certificate
from the fixture is injected into the provider. Refinement and owned queries lack
a historical Cadrum equivalent and are explicitly unpaired. P1 does not cover the
newer source-backed PCB producer; its parent functional gates remain required.

See the [runbook](../../../../docs/research/cad/integrated-cad-performance-runbook-2026-10-18.md)
for execution, prerequisites, source/profile bindings and limitations.

U1 now freezes the **unmodified actual product download** in `U1.boardstudio`, with
its ordinary input/endpoints in `U1-scope.json`. The accepted 2×2 rigid project has
four DCS/MX caps, four generated Case/foam bodies and the nominal1.6mm physical
PCB. Standard archive export embeds its two original used bundled STEP assets;
all authored geometry, source references and warnings are retained. Current real
import/render, mechanical STEP and full Runtime four-STL/manifest/ZIP/download are
validated. This U1 page block is distinct from K1's25-cap worker transport and
does not claim source-backed PCB, printable legends or the historical page passed.
The historical source already has these Runtime export actions; actual old loader,
geometry and export admission await its real full package in a new root-owned
measurement checkout. The protected old lane remains read-only.
