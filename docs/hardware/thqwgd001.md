# THQWGD001 library qualification

BoardStudio includes **nominal, unqualified** THQWGD001 assemblies. Library
presence and a successful preview do not certify fabrication or firmware.

The source is Taro Hayashi's [THQWGD001 repository at
78e1c42dbebca1a9e28cf057d7d84f7eb786aa15](https://github.com/Taro-Hayashi/THQWGD001/tree/78e1c42dbebca1a9e28cf057d7d84f7eb786aa15).
Assets retain CC BY 4.0 attribution, source hashes, and unchanged footprint
source. Portable STEP references replace the author's absolute paths on output.

## Available selections

- THQWGD001: rotation only; reversible footprint and nominal #1 assembly.
- THQWGD001C: separate 2-pin and 4-pin tactile packages, reversible footprints
  and matching nominal #1 assemblies.
- Each family has routed and drilled mounting-cut helpers as separate parts.
  These are **not automatically attached** when replacing a key. The routed
  source helpers contain open Edge.Cuts construction geometry and need reviewed
  integration into closed board contours. Do not treat their source lines as
  machining-ready slots.

Use the Parts library for standalone placement, or select a matrix key and use
its Key Assembly selector for a switched C variant. Rotation-only variants cannot
replace a key without an independent press pair. The matrix key keeps its logical
identity, bindings, and pose. Undo and saved reload preserve the replacement.

The [pinned required-parts lists](https://github.com/Taro-Hayashi/THQWGD001/blob/78e1c42dbebca1a9e28cf057d7d84f7eb786aa15/README.md)
also describe #1/#2 encoder and flat/curved wheel alternatives. Those labels do
not identify manufacturers. Wheel-specific assembled selections remain pending;
BoardStudio does not substitute a loose constituent model into an assembly with
an invented datum.

## Nominal model evidence

The STEP mounting datum is retained: no XY recentering or lifting the lowest lead
to zero. C variants use their published identity model transform. Non-C uses a
candidate zero-degree rotation: the published reversible +90-degree stanza puts
its modeled leads on the wrong axis relative to the footprint's holes. This
adaptation remains gated pending independently qualified hardware alignment.

| Bundled model | SHA-256 |
| --- | --- |
| THQWGD001-rotation.stp | 6ae146dcff8704aed80e95e8aa4693df567c55ea88027825a98bfcae4b0b92eb |
| THQWGD001C-2pin.stp | 6cc61fd27e32e4f6fc202b96f26ac1f0e6edb6afce58e36d09a91eac5eeeb71d |
| THQWGD001C-4pin.stp | 9ad6b52629349e8ef9a28b6bdc07f5f8f787266d24b025770e9d46c71be1c7c0 |

Nominal static bounds are recorded in each mechanical profile. The C4 model's
19.419158001 mm width can overlap another copy at 19.05 mm MX pitch; the 1U label
never overrides a fit finding. Findings highlight the intersecting bounds.
Public CAD import tests compare the actual mesh extrema to the independent
measurements within 0.03 mm. The kernel-reported bounds are wider for these curved
solids and are checked for containment; they are not used as tight fit evidence.
These rectangular occupied bounds are conservative, not exact swept solids or
measured service envelopes. Parts without an occupied-volume profile do not
receive this model-bound comparison.

Back-side volumes use the host PCB surface and thickness. This coordinate
convention is not independent proof of an assembled back-side contact map.

## Output gates and remaining evidence

The inspector exposes blockers per output. Public PCB/footprint export checks
footprint and electrical blockers during preparation **and completion**. Nominal
preview returns projected geometry through a separate path; its preparation
cannot be reused to bypass fabrication qualification. Case generation retains
mechanical blockers. ZMK handoff carries a compact selected-part qualification
snapshot and blocks unresolved firmware facts.

Remaining source facts and digital configuration:

- Orderable encoder identity, source-backed contact mapping on both sides, and
  resolution of the ordinary #2 versus reversible A/C pad ambiguity. Physical
  continuity testing is deferred.
- Selected tactile identity and toleranced package fit, especially the unknown
  2-pin package; named 4-pin drawings alone do not identify purchased hardware.
- The source #1 model's nominal back transform is now checked against the
  reversible footprint hole locations and board thickness. Back-side contact
  continuity, modeled solder-mask/tool access, assembled travel, plate openings,
  and toleranced fit remain separate source/configuration gaps. Production
  variability is deferred.
- The drilled helper's eight published NPTH slots now have a rotated public
  export/preview check. Its public footprint gate records the unresolved
  helper-to-encoder pose assumption and selected-fabricator slot rules; it does
  not report a closed-contour blocker. The routed helper remains gated because
  its pinned Edge.Cuts geometry is open; BoardStudio does not invent a closing boundary.
  The pinned `PCB_Sample/THQWGD001C.kicad_pcb` (SHA-256
  `5f2bfd4a7b142eeee038d3cefcc4815ac8e229007ab0ef9b6f4153485b0d1de7`) contains
  one closed board-perimeter loop made from 14 Edge.Cuts primitives, no `SlitC`
  helper placement and no independent route contour, so it cannot establish that
  helper's location or closure.
  Minimum slot/web rules still need an explicit selected-fabricator profile.
- Wheel-specific assembly datum, press/rotation swept bounds, and service bounds.
- Encoder pulses, detents, direction, and documented driver configuration; physical firmware validation is deferred.

The creator's [integration article](https://note.com/taro_hayashi/n/nf608af2136d1)
documents A/B rotary outputs, C common, and separate press contacts 1/2. Repeated
physical pad IDs retain the published logical numbers and masks. No presumed
mirrored electrical mapping or numerical encoder timing is supplied.

## Digital acceptance and human review

The agreed acceptance path is automated checks plus human review in BoardStudio.
Physical hardware testing is unavailable and is not a completion requirement for
this implementation. Acceptance certifies the supported digital design and its
recorded assumptions; it does not certify manufactured hardware.

| Area | Automated evidence required | Human review in the app |
| --- | --- | --- |
| Model bounds | Import each pinned STEP, verify finite mesh and tight extrema against independent source measurements; check conservative bounds contain the mesh | Inspect all three variants for missing bodies, unexpected scale or floating geometry |
| Assembly datum | Compare transformed lead/hole positions, PCB surface and thickness on front/back; preserve the source datum | Check leads, holes, wheel axis and PCB crossing from top, bottom and side views |
| Fit | Check nominal overlaps at MX and Choc pitches, rotated/back-side placements and a clear control; report precisely located intersections | Select findings and confirm the highlighted region matches the interference |
| Mounting and exports | Verify drilled slot holes, masks/pads and portable model transforms survive export; compare exported geometry with preview; keep any open routed contour gated unless source-backed integration closes it; run DRC with selected fabrication rules | Inspect the mounting cuts and plate/case openings; confirm the chosen orientation and any paired-helper placement assumption |
| Editing | Verify standalone placement, matrix replacement, bindings, Undo/Redo and saved reload | Perform placement/replacement and reopen the saved board |
| Electrical and firmware | Preserve published pad roles and repeated pads; test matrix/direct-press planning and explicit configuration errors | Review selected press scan mode and rotary configuration; identify assumptions requiring confirmation |

Ten public Rust tests cover output gates, source contact preservation,
front/back exported hole and model placement, MX/Choc nominal fit at 0/90 degrees,
back-side fit, drill-helper slot geometry through rotated export and preview, and
separate nominal preview. A regression asserts that only the routed helper gets
the open-contour gate while the drilled helper gets a reviewable pose assumption.
Nine real STEP-import tests compare pinned bounds,
geometric lead-tip positions against the imported source hole centers, and front
and back transformed extents against the independent FreeCAD oracle. Five THQ
browser workflows cover variant discovery, replacement, standalone placement,
pitch/rotation finding behavior and precise highlight navigation. These checks
validate the digital geometry and transforms; they do not certify physical
continuity, motion or manufacturing tolerances.

The drill helper is still an independently placed library footprint; an
automatic companion-placement action is not implemented. The pinned source does
not state its transform relative to an encoder. Any future “add matching drilled
helper” action must show shared-origin placement as a user-reviewable
configuration assumption. The routed helper cannot be made closed by endpoint
snapping: the source contains a deliberate opening, not a small numeric endpoint
gap.

For each review, record the commit, saved project, variant, PCB thickness,
orientation, pitch, screenshots and a pass/fail result for each applicable row.
Keep failures as reproducible saved projects. Human review remains **pending**
until the user records it; automated browser interaction is not human sign-off.

Use three outcomes for unknowns: resolve from pinned source evidence, expose an
explicit user-supplied configuration, or retain the affected output blocker.
Do not erase every blocker to enable review: nominal preview already supports
inspection separately from fabrication. Physical continuity, production
variability and device operation remain deferred hardware assumptions, not a
request to obtain hardware before continuing development.
