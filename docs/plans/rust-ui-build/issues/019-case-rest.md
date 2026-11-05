# 019: Case rest

Status: resolved
Owner: lane/019-case-rest (.worktrees/lane-019-case-rest)
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The other Case files (bodies, collision checks, viewer controls, layers, panels) and the Case workspace host. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [x] Every Case row other than the settings file has zero raw controls and zero `Board*` uses
- [x] Case generation, cancel and retry controls keep their readiness behaviour; the Case workspace works in both Themes and both Appearances at the three widths, with screenshots in the outcome
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Answer

Migrated all seven Case files outside the settings file plus the Case workspace host, in
TDD slices with role+name tests written first (RED) and passing after migration (GREEN).
Lane branch `lane/019-case-rest`, merged to feat tip `b47d63174` (018+021) in `9bda7b73`.

### What changed (production)

- `case_collision.rs`: disclosures to Collapsible, clearance to Field/InputGroup number
  entry, buttons to Secondary Buttons; errors to Alert, notices to Callout, findings to
  Item rows with provenance Collapsibles. Pending status stays inline.
- `case_assembly_layers.rs`: trigger to CollapsibleTrigger with ChevronDown; select rows
  and eye toggles to labelled ToggleGroups; icon-only eyes gain Tooltips (mirrors the
  landed shared `LayerToggleButton`). Unavailable-row titles drop (states render inline).
- `case_bodies.rs`: body tabs to a labelled ToggleGroup; kind/mount selects to
  Field+Select; number fields to Field/InputGroup compositions with the unchanged
  draft/commit/Escape/Undo logic; Mounting/Gasket disclosures to controlled
  Collapsibles; buttons to Secondary Buttons; notices to Callouts, the global failure
  to Alert. Disclosure h3s flatten into trigger labels.
- `cad_presentation.rs`: model delivery to Collapsible with Alert failures; Live
  preview `role=switch` to a labelled Checkbox (no Switch component exists — the one
  role change, documented here); Update/Cancel/Export to Secondary Buttons.
  Readiness, cancel and retry behaviour unchanged.
- `case_viewer.rs`: keycap error to Alert with Retry Button; findings to Collapsible
  with Item rows. Canvas and preview lifecycle untouched.
- `shared_viewer.rs`: DesignViewToolbar to View-mode and Footprints ToggleGroups;
  inline camera buttons to Secondary icon Buttons with Tooltips; display, assembly,
  hidden-lines and edit-mode controls to labelled ToggleGroups with icon items and
  Tooltips; explode/section to Sliders, section plane to Select, show-plane to
  Checkbox — all mirroring the landed non-inline branch. Toolbar hints and the
  native viewer color well stay as-is (020 precedent).
- `case_workspace.rs` (host): tree disclosures to Ghost icon Buttons with ChevronDown
  and Tooltips, row selects to Secondary Buttons (016 pattern), visibility eyes to
  ToggleGroups with Tooltips; Assembly setup to Collapsible; Half connection to
  Select; PCB design to Select-or-Combobox by the 12-option rule; flip/Visible to
  Checkboxes; guidance to Callouts; Reset colour to Button. Native Part colour well
  retained per mapping section 4.
- `case_controller.rs`: production untouched (0/0 controls); its session tests now
  find fields by role+name.
- CSS: deleted only exclusively-owned dead selectors; kept shell/layout/swatch
  rules; added `:where(:not([data-slot]))` exclusions where legacy element rules
  would hit new controls (design view group, physical-setup labels, mirroring 016).
- New repo browser test `scripts/test-case-rest.py`: packaged journey (new project,
  matrix, Case, authored body, Body-type keyboard floor, scene mount, readiness
  observation) plus the capture matrix below.

### Inventory (acceptance 1)

Zero `Board*` and zero raw controls in all seven files except test fixtures and two
mapping-sanctioned native `input[type=color]` wells inside Field+Label (mapping
section 4; 020 precedent). Evidence `.evidence/019/inventory.log`.

### Commands and results

- `wasm-pack test --headless --chrome web/crates/case --locked --lib`: **100/100**
  post-merge (91 own incl. 5 new bodies + 1 toolbar tests; 018's 9). Per-slice RED
  logs (`red-*.log`) and GREEN logs retained; combined-run failures after a
  panicking RED test are documented pollution, solo runs are the clean signals.
- `wasm-pack test ... web --bin boardstudio-web --
  presentation::case_workspace::mounted_live_scene_tests`: **4/4** (2 new: tree +
  physical assembly incl. the 13-board Combobox path).
- `cargo test --locked -p boardstudio-web-case --features test-support`: 34/34.
- `cargo clippy` (wasm all-targets, `-D warnings`) for case and web: clean.
- `cargo fmt --all --check`: clean. `check-wasm-tests.py`, `check-doc-links.py`
  (571 files): clean. `check-old-css.py`: 608 selectors remain (global 024 gate).
- `python3 scripts/check.py test`: passed (2104 baselined + 6 new native tests from
  the 018/021 landing — baseline update is their follow-up, not this lane's).
- Page: `build-web.py --providers-only --fast` then `--fast --skip-providers`,
  served on 48119; `test-case-rest.py` PASS with zero browser errors.

### Screenshots (acceptance 2)

`.evidence/019/captures/` (32 PNGs + `captures.json`): Case workspace in
Original/shadcn x light/dark x 1440/1050/390, plus 390 narrow-sheet Inspector
shots, in both unconfigured (bodies editor) and configured (settings +
generated-stack note) states. Representative wide/mid/narrow captures in all
four combinations inspected: tree, viewer toolbar, 3D scene, collision checks,
generation footer (Update enabled, Cancel idle-disabled), bodies editor,
settings, and narrow sheet all render correctly.

### Decision revisions and notes

- Live preview `switch` becomes `checkbox`: no Switch component in the adopted
  set; on/off behaviour unchanged. No source-doc change (mapping sends
  BoardCheckbox to Checkbox).
- Findings rows use Item without Badge: no severity data exists to badge;
  inventing severity would be speculative.
- Disclosure h3s (Mounting/Gasket) flatten into trigger labels: headings lose
  outline semantics inside button triggers regardless; mismatch h2/h3 kept.
- Merge conflict (one, `tailwind.css` board-field rule): 021 removed the
  model-axes half, this lane the measures half — deleted the fully dead rule.
- Post-merge fallout fixed in-lane: the case11 workspace test asserted 018's old
  `li` diagnostics; now finds `Show finding {id}` by role+name (test-only,
  same assertions). No 018 production code touched.
- Limits: physical coarse-pointer and screen-reader verification remain open
  (global gates); color wells exercise callbacks only, not the native dialog;
  keycap retry/findings UI has no dedicated wasm test (same roles/names/text).

