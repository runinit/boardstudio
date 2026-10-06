# Rust core

The browser runs Core inside the `web` crate's core-worker build
(`python3 scripts/build-web.py --providers-only`). `CoreEngine.request(json)` accepts and returns the JSON protocol
from `contracts`. Preview replies contain projected scene geometry while
`document` remains committed. Commit, undo, and redo each advance revision.

Enabled Rhai scripts run atomically when opening or replacing a document. The
only registered output functions are:

```rhai
part("main", "left-esc", "switch", "SW1", 0.0, 0.0, 0.0);
rect("main", "edge", 0.0, 0.0, 80.0, 30.0, 3.0, "add");
```

Object IDs are `script/<script-id>/<group>/<local-id>`. Script, group, and
local IDs use letters, digits, `_`, or `-`. A moved generated part keeps its
visual pose when the document is reopened. Scripts have operation, call depth,
source size, string, array, map, and emission limits. No file or network
functions are registered.

Native outline timings: `cargo test --release --manifest-path core/Cargo.toml
--test performance -- --ignored --nocapture`. This measures Rust transactions,
not browser input-to-paint latency.

Outline paths are cached per feature. A part move rebuilds only envelopes that
reference that part; ordered boolean composition still runs for every feature.
Rounded corners use a 0.05 mm maximum chord error with at most 1024 segments per
quarter arc. Compare full and cached resolution with `cargo test --release
--manifest-path core/Cargo.toml --lib -- --ignored --nocapture`.

`SceneDelta.boardContours` resolves each board's `outlineIds` in authored order.
PCB readiness uses these board contours and board references; it does not depend
on unrelated design outline features. Case readiness checks the selected board,
wall dimensions, mounts, material, and gasket depth.

`set-matrix` generates members with IDs `matrix/<id>/r<row>c<column>`.
Coordinates use positive pitch, optional X/Y mirror, then matrix rotation about
its origin. Moving a matrix member sets `properties.layoutOverride=1`; later
matrix edits retain that member's pose. Shrinking a matrix removes retired
members from boards, envelopes, and net pins in the same undoable transaction.
Sparse `cells` override individual coordinates; omitted cells are enabled.
Row, column, and cell offsets adjust local positions before mirroring and
rotation. Cell assemblies create stable companion IDs under the host key ID.
`edgeGap` defaults to 1 mm on each axis and describes visible keycap clearance;
it does not change the center pitch. Preview replies contain a scene without a
document. Move previews save only affected poses and properties; matrix
previews save the collections they may edit. Both restore committed state
before replying. The committed document remains available through snapshots.
When `diodes: true`, the core requires the `matrix-diode` definition, adds a
`/diode` part on the back at local (+6,-10) mm for each enabled cell unless
`cell.diode` is false, and maintains
stable row, column, and switch-link nets. This mode requires switch pad IDs
`one` and `two`, plus diode pad IDs `anode` and `cathode`.
RGB LED assemblies require pads `vdd`, `gnd`, `din`, and `dout`. The core
maintains matrix-owned power nets, input/output endpoints, and deterministic
DIN/DOUT chain across enabled cells. A `set-matrix` edit may include new
definitions; registration and placement share one undo step.
