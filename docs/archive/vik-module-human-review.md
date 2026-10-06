# VIK module app review

> Archived React workflow. The `app/` frontend and the Playwright tests named
> below were retired at `3cdeb2ac2`; these commands are not runnable in the current
> checkout. Retained review scenarios do not establish Dioxus or hardware
> qualification. See [VIK status](../hardware/vik.md) and
> [current checks](../../README.md#checks).

Open **Project → Demo keyboards → VIK module review · above and below**. The app creates an editable local project with pinned-source splitter snapshots above and below the host board, a source-backed EC11/VIK rotary module, a rotation-only wheel, C2/C4 THQ encoder modules replacing matrix keys, layered key behavior, and an independently embedded DRV2605L reference circuit. Opening the demo does not overwrite another saved project. Rename it or use **Save project copy…** to keep a separate review copy.

After selecting a module in Parts, its review note shows the known assumptions. The chosen anchors and 3 mm face gaps are illustrative. Board contours and recorded board thicknesses are source data. The displayed 6 mm outer, 2.8 mm hole, 3 mm high standoffs are designer-selected examples, not vendor fastener dimensions. Component, cable and actuator envelopes are incomplete. The upstream catalogue labels these modules Untested. App review checks presentation, bounds, findings and saved behavior. It cannot establish continuity or prove firmware or fabrication readiness.

## Review in the app

1. Open **PCB** and inspect the **Mounted modules** layer group. Footprints and their source silkscreen are visible by default; module board outlines, clearances, mounting holes, standoffs, fabrication art and findings have independent toggles. Toggle them separately and confirm the host board's outline, copper, pads, holes and references stay independent. Clicking a module footprint opens Parts with that exact placement selected. These overlays are uneditable source previews and do not add module pads or nets to the host PCB.
2. In the module inspector, inspect both splitters' two source holes and resolved board standoffs, including their host-frame positions. The demo's example standoffs use OD 6 mm, ID 2.8 mm and height 3 mm; they are designer-authored test values, not sourced hardware dimensions. Switch **Attachment** to **Fixed to case** in a disposable copy to inspect how the existing support fields configure case rings instead. Do not save that exploratory change into the original demo.
3. Open **Parts**, select **module to split one VIK in connector to two VIK outs**, and check the two entries in **Placement**. Confirm the first is above and the second is below, and inspect their XY anchor, rotation, face and gap values. Change a value, save, Undo, Redo, reload and confirm the instance identity and placement return as expected.
4. Expand **Available 3D models**. Compare each displayed X/Y/Z source bound in millimetres with the linked pinned source asset. Bounds are in asset axes; they do not confirm model-to-PCB alignment or fit. Confirm the unreviewed alignment label remains visible.
5. Open **Case** and inspect each module board and the finding list. Check whether the incomplete component and cable profiles produce blockers. The demo deliberately leaves physical envelope gates unresolved; a clear preview is not a claim of clearance.
6. Open the keymap pane. Confirm C2 and C4 occupy matrix positions, the base layer includes mod-tap and layer-tap actions, and the navigation layer includes a macro with a wait step. Inspect rotary actions for the standalone wheel, EC11 VIK module and matrix encoders. The EC11 source does not provide pulses or actions per rotation. ZMK defines encoder `steps` as pulses per complete rotation; check the [ZMK encoder documentation](https://zmk.dev/docs/config/encoders) and confirm both values for the selected hardware before expecting a firmware-ready profile.
7. Return to **Parts** and select **Haptic Feedback using DRV2605L controller**. Expand **Use circuit on PCB** and verify the copied circuit remains separately listed. Its board parts and local nets should remain distinct from mounted daughterboards. Inspect the source adaptation note and unresolved actuator/driver gates.
8. Save, reload, then use **Save project copy…**, start a new project and import that archive. Confirm module instances, host connector footprints, standoff settings, matrix replacements, explicit rotary-profile edits, keymap actions and the embedded copy remain. The VIK connectors are placed as source-backed host footprints with empty MCU assignments; no routed cable or signal continuity is implied. Try **Export ZMK source**: the fixture has no reviewed host controller/GPIO wiring, so firmware export must remain blocked. The EC11 profile's missing pulse/action measurements remain unknown even if a test or user supplies values in a copy.
9. In **Case**, use a module failure's **Select affected geometry** action and confirm the app focuses the failure region. This is also exercised by the separate `vik-case-findings.spec.ts` browser regression.

## What to record

- Above/below orientation or face-label confusion, including the module front/back direction and case support-ring contact behavior.
- Whether footprints, silkscreen, board outlines, clearances, holes or standoff layers are missing, hidden by host layers, incorrectly placed, or unexpectedly included in fabrication output.
- Whether a board or candidate model appears at the wrong scale, position or rotation. Record the asset name and its displayed source bounds.
- Any missing or unexpected finding when module boards overlap the host, each other, or case solids.
- Whether undo, reload, key assembly replacement and circuit removal preserve all unrelated keyboard and module data.
- Any place where the UI appears to imply a measured clearance, completed electrical connection or supported driver that the source evidence does not provide.

## Automated companion

`app/e2e/vik-module-review.spec.ts` opens the same gallery demo, checks the PCB module-layer defaults/toggles and parent selection, pinned snapshots, model-bound labels, matrix encoder replacements, keymap layer bindings, placement save/Undo/reload, embedded-circuit identity, archive round-trip and blocked firmware export. `app/e2e/vik-case-findings.spec.ts` separately verifies geometry navigation for a module/case collision. Run them after the current app and Rust/WASM bundles have been built:

```sh
pnpm --dir app exec playwright test e2e/vik-module-review.spec.ts
```
