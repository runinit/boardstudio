# 011 executable protocol and untimed validation

Successor to preparation `2c680453`; root has **not** cleared acceptance timings
or builds while007 is active. No ticket is closed. CAD, importer, worker, Runtime,
dependencies and the authoritative analysis contract/fixture are unchanged.

## Binding and validation route

The harness checkout and served checkout may differ. `validate` uses the existing
parent `.evidence/qualification-v2-integration/artifact-receipt.log` directly; it
never fabricates a build-time record or requires a new build. The receipt identifies
tested source `01680d8410ce3cb59e751871f97e669d0a01640f`, checked release/no-opt CAD
and Host worker, and debug Dioxus. Parent `d9b58b6d` is a docs-only successor.
The route checks that relationship, the official CAD input stamp, all four retained
provider/Host JS/WASM byte-equality/size statements, and the actual packaged index.
Existing receipt helpers bind the current package/site, harness, input Git objects
and source/diff before/after every observation. Exact logs remain source-bound;
source/driver/artifact drift invalidates the capture.

`validate` runs **one** observation, has no growth/repetition sequence, writes
`diagnostic.csv`/validation logs and never writes `runtime.csv`, build timing or
memory benchmark data. Every value is diagnostic, not a benchmark result. The
report reducer explicitly rejects these captures. `runtime` still requires root's
independent admission/quiet-window note and a successful matched build capture.

From this lane, using the existing cached browser prerequisite (no install):

```sh
export NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules
HARNESS=/home/chris/.t3/worktrees/ts-boardstudio2/t3-1d37b199
PARENT=/home/chris/.t3/worktrees/ts-boardstudio2/t3-fad4a930
RECEIPT="$PARENT/.evidence/qualification-v2-integration/artifact-receipt.log"
python3 -B "$HARNESS/cad/scripts/measure-cad.py" validate --checkout "$PARENT" --source-head d9b58b6da2cb9a5438136cfb052be354dd14acdd --profile checked --workload C1plate:preview --artifact-receipt "$RECEIPT" --output "$HARNESS/.evidence/011-next-C1preview"
```

Use a **new** output directory for each command. Other finite worker endpoints:
`C1plate:step`, `V1switch:import`, `V2vendor:import`, `V2vendor:refine`,
`Qvendor:default`, `Qvendor:tight`, `K1keycap:print`. Requests use the
[frozen DTOs](../../../cad/test/fixtures/performance/README.md), original vendor
bytes/default repair selection and the **unchanged** shared analysis fixture.
Queries mint actual worker-owned references; default0805 retains its authored1mm
source band, while tight0.01 remains Unknown from the existing precision budget.
Vendor refinement is separate current-only work, not an old-API speedup. Preserve
actual movement, selected allowance and source proof as distinct fields.

## U1 actual page and downloads

[U1.boardstudio](../../../cad/test/fixtures/performance/U1.boardstudio) is the
unmodified real Runtime project download, including two original used bundled
assets. [U1-scope.json](../../../cad/test/fixtures/performance/U1-scope.json) pins
its identity, format7/revision3, ordinary2×2 rigid design, four DCS/MX caps,
viewport1513×1043, selected Main board case assembly, and
Assembled/Shaded/Isometric/Fit actions. The format-7 archive is the unmodified Runtime download from the unbound
`.evidence/ticket006-u1-unbound/freeze` capture; its site artifacts match the
checked build at `80f4b6adecc9fb6b1e4c36842b536ddc89c1f590`. Input geometry
and warnings are retained.
This is the common analytic physical PCB; source-backed PCB/ring-island proofs
remain the parent's separately delivered scope. K1's25-cap worker request remains
separate from U1's complete four-cap page export.

The existing product flow creates the matrix, configures the rigid case via the
guide, hides that guide to reveal the inspector, and opens Matrix profiles to
choose DCS. `U1:page` imports the frozen archive through the **real** Library file
input. No IndexedDB document injection, renderer replacement or synthetic STL
writer is used. Capture requires the accepted-revision CAD reply, Exact case
geometry ready, a real WebGL2 draw following Fit, `finish()` completion, a visible
nonzero canvas and screenshot. Selection, viewport, camera actions, worker
requests/replies and accepted project are recorded. All export actions must leave
that accepted document unchanged.

The actual Export workspace actions produce:

- **Generated mechanical package:** `assembly.step` contains all four generated
  Case/foam bodies plus physical PCB; four part STEP/STL files, outlines and
  `assembly.json` retain their real Runtime identities/diagnostics.
- **Keycap STLs:** all four fused keycaps, binary STLs, `keycaps.json` with the
  captured revision/mm units, ZIP packaging and actual browser download.

```sh
python3 -B "$HARNESS/cad/scripts/measure-cad.py" validate --checkout "$PARENT" --source-head d9b58b6da2cb9a5438136cfb052be354dd14acdd --profile checked --workload U1:page --artifact-receipt "$RECEIPT" --output "$HARNESS/.evidence/011-next-U1"
PYTHONPATH=/usr/lib/freecad/lib python3 -B "$HARNESS/cad/scripts/check-cad-page-downloads.py" "$HARNESS/.evidence/011-next-U1" --build-run "$HARNESS/.evidence/011-next-build" --freecad
```

The checker validates full ZIP inventories/CRC, original embedded asset SHA,
accepted inputs, manifest identities/revision/units, and STEP bytes equal to actual
completed worker buffers. Independent STL decoding checks every worker vertex and
triangle normal, then reuses the existing print regression's closed/connected/
outward topology and analytic roof/cavity/stem/socket probes. U1's authored18mm
bounds use the same retained1e-5mm coordinate gate; the original18.2mm regression
fixture is unchanged. Truncation is a negative control. Optional **already
installed** FreeCAD reads emitted STEP independently: validity/closure, five
assembly solids/four single-solid part files and the PCB's1.6mm z span. This is
one-time transport/file QA, not a new required C++ dependency or a substitute for
the retained008 independent source-accuracy proofs. Do not install/provision the
reader if absent; record that limit.

`U1:freeze` remains a validation-only authoring command for reproducing the ordinary
flow; it writes only a new capture directory. The committed archive is the frozen
input. Do not overwrite it automatically or regenerate random IDs during timings.

## Build observations after root clearance

The retained [plan](integrated-cad-performance-plan-2026-10-18.md) literally said
“three alternating paired build cycles per profile,” yielding36 invocations. Its
new dated root revision explicitly supersedes that repeat count. The default
schedule now proposes **one** matched clean/no-op/incremental cycle per backend
per profile:12 single observations across checked and release. Shipped optimized
release measures shipping cost; checked/no-opt describes development cost and
matches existing functional artifacts. These are separate groups, never size or
latency deltas between mismatched profiles. Repeat only to resolve observed
variability or material regressions; `--build-cycles 3` preserves the old upper
proposal. A single observation is descriptive, not significant. The finite runtime
five-pair cold/warm protocol and session-level statistics remain unchanged.

```sh
python3 -B "$HARNESS/cad/scripts/measure-cad.py" plan --output "$HARNESS/.evidence/011-next-plan"
python3 -B "$HARNESS/cad/scripts/measure-cad.py" plan --build-cycles 3 --output "$HARNESS/.evidence/011-optional-upper-plan"
```

Plans never execute automatically. Later build commands remain `build --checkout
"$MEASURE" --source-head "$ACCEPTED_HEAD" --profile checked --pair 1 --state clean
--output "$HARNESS/.evidence/011-build-clean"`, then `--state noop`, then the same
owned comment appended once at EOF of registered `cad/wasm/src/lib.rs` and
`--state incremental`. The exact additional bytes are
`// BoardStudio CAD performance incremental rebuild probe.` followed by one LF.
Apply it **before** capture in the dedicated measurement checkout; keep the harness
checkout fixed and separate throughout the matched cycle. Clean/no-op samples must
have no source patch or untracked source inputs. The incremental sample retains the
same accepted HEAD and permits only this appended comment, with its complete Git
diff and source digest preserved in both binding logs. No other source revision or
patch may be pooled into the matched comparison.

After root clears builds/timings, the guarded edit is:

```bash
python3 -B - "$MEASURE" <<'PY'
from pathlib import Path
import subprocess
import sys
root = Path(sys.argv[1])
relative = 'cad/wasm/src/lib.rs'
base = subprocess.check_output(['git', 'show', f'HEAD:{relative}'], cwd=root)
comment = b'// BoardStudio CAD performance incremental rebuild probe.\n'
path = root / relative
assert base.endswith(b'\n') and comment not in base
assert path.read_bytes() == base, 'preserve any existing work; do not overwrite it'
path.write_bytes(base + comment)
PY
```

Restore only those exact appended bytes afterward, checking the file still equals
`base + comment` before writing `base`; preserve any unexpected edits for root.
The capture and reducer share the build-binding check: every successful build row
requires complete before/after bindings matching its backend/profile, stable source,
harness/workloads and resolved output paths, a current official provider stamp and
present packaged artifacts afterward. Output creation, artifact/stamp changes and
output-directory existence changes are expected; source/harness drift is rejected
even when the build command exits zero. Rejected captures retain both logs,
`binding-rejection.log` and a `binding-rejected` CSV row, and return nonzero.
Reduction independently checks these rules, a fixed source/profile per backend,
common harness/workload identity across the matched profile's builds and fixed
provider inputs per base/incremental patch. Earlier build rows without the required
bindings cannot be admitted by a successful command exit or retroactive metadata.

Checked calls normal `build-web.py --fast`; release calls normal `build-web.py`.
Root must use fresh dedicated outputs for clean observations. The tool never
cleans/discards/edits source and retains normal jobs/debug flags. Preserve the exact
owned diff; restore only that edit after capture. Stage stdout timestamps, total
wall seconds, raw sizes/deterministic gzip bytes and50ms per-PID memory samples
remain available for **later authorized** captures. Only sum the packaged-site
size group; separate copied provider/worker views are not extra shipped bytes.

## Historical prerequisite and remaining gates

The protected `t3-2088b1aa` lane remains detached/clean/read-only at
`6c9c4867755bd2316f751783729a3ff9778fc04a`. Its bound20,294,309-byte **no-opt provider**
and old15-check smoke do not supply a packaged old page/Host worker. No actual old
worker/loader/render/download pass is claimed. Static source confirms the old
Runtime has both mechanical and keycap export actions; capability parity and
geometry/output quality still require actual admission.

Root must provision a **new T3-owned historical measurement checkout** at that
commit, with distinct resolved root/CAD targets and real output/cache directories,
then build its full page/Host/provider packages for the selected profile. Reuse
only the verified archive/download prerequisites: pinned final image digest
`sha256:569918e5874f8908957a76e2b2df7518d9c7886f5d224c5536ec2ca9896fd2f0`,
WASI SDK33/Rust1.98/bindgen0.2.129, and the verified OCCT `8_0_1_rev2` prebuilt
archive from the retained plan. Verify provenance/effective container jobs first;
only then use `CADRUM_IMAGE_READY=1` with normal historical build helpers. No
automatic SDK/image fallback build, protected-lane mutation, source restoration
into parent, or provisioning happened here. Optimized old products remain absent.

Root must next accept post-007 movement/default-budget behavior, freeze functional
source, refresh official matched artifacts/admissions and name a quiet window.
Pause competing builds/checks, verify targets/CPU/load/governor, serialize every
build and preserve/invalidate an interfered pair. Only then run acceptance samples.
Current-only source/qualification classes stay separate, unsupported classes and
containment remain Unknown, and no general Boolean requirement is added.

## Validation evidence

This lane's `.evidence/011-validation/` retains the exact commands/bindings and
results. `page-freeze-v1`/`v2` preserve the real UI failure: guide visibility hid
the inspector/profile controls. `page-freeze-v3` passes the corrected actual
product workflow and independent emitted-file QA. The first K1 print transport
completed, but its seal was rejected because a concurrent harness commit changed
driver identity; that rejected capture is preserved and is not an accepted sample.
Final focused tests and source-frozen replay results are recorded in the lane's
handoff. No Cargo/wasm-pack/Dioxus/container build, install, full benchmark, parent
edit, review or migration completion is claimed. Ticket011 status stays unchanged.
