# VIK module app review

Open **Project → Demo keyboards → VIK module review · above and below**. The app creates an editable local project with pinned-source splitter snapshots above and below the host board, a source-backed EC11/VIK rotary module, a rotation-only wheel, C2/C4 THQ encoder modules replacing matrix keys, layered key behavior, and an independently embedded DRV2605L reference circuit. Opening the demo does not overwrite another saved project. Rename it or use **Save project copy…** to keep a separate review copy.

After selecting a module in Parts, its review note shows the known assumptions. The chosen anchors and 3 mm face gaps are illustrative. Board contours and recorded board thicknesses are source data; component, cable, support and actuator envelopes are incomplete. The upstream catalogue labels these modules Untested. App review checks presentation, bounds, findings and saved behavior. It cannot establish continuity or prove firmware or fabrication readiness.

## Review in the app

1. Open **Parts**, select **module to split one VIK in connector to two VIK outs**, and check the two entries in **Placement**. Confirm the first is above and the second is below, and inspect their XY anchor, rotation, face and gap values. Change a value, save, Undo, Redo, reload and confirm the instance identity and placement return as expected. Case-attached placements expose designer-entered support rings around source PCB mounting holes; no support dimensions are assumed by this fixture.
2. Expand **Available 3D models**. Compare each displayed X/Y/Z source bound in millimetres with the linked pinned source asset. Bounds are in asset axes; they do not confirm model-to-PCB alignment or fit. Confirm the unreviewed alignment label remains visible.
3. Open **Case** and inspect each module board and the finding list. Check whether the incomplete component and cable profiles produce blockers. The demo deliberately leaves physical envelope gates unresolved; a clear preview is not a claim of clearance.
4. Open the keymap pane. Confirm C2 and C4 occupy matrix positions, the base layer includes mod-tap and layer-tap actions, and the navigation layer includes a macro with a wait step. Inspect rotary actions for the standalone wheel, EC11 VIK module and matrix encoders. The EC11 source does not provide pulses or actions per rotation. ZMK defines encoder `steps` as pulses per complete rotation; check the [ZMK encoder documentation](https://zmk.dev/docs/config/encoders) and confirm both values for the selected hardware before expecting a firmware-ready profile.
5. Return to **Parts** and select **Haptic Feedback using DRV2605L controller**. Expand **Use circuit on PCB** and verify the copied circuit remains separately listed. Its board parts and local nets should remain distinct from mounted daughterboards. Inspect the source adaptation note and unresolved actuator/driver gates.
6. Save, reload, then use **Save project copy…**, start a new project and import that archive. Confirm module instances, matrix replacements, explicit rotary-profile edits, keymap actions and the embedded copy remain. Try **Export ZMK source**: the fixture has no reviewed host controller/GPIO wiring, so firmware export must remain blocked. The EC11 profile's missing pulse/action measurements remain unknown even if a test or user supplies values in a copy.
7. In **Case**, use a module failure's **Select affected geometry** action and confirm the app focuses the failure region. This is also exercised by the separate `vik-case-findings.spec.ts` browser regression. The review project intentionally has no configured VIK host connection; do not infer electrical compatibility from the catalog or geometry preview.

## What to record

- Above/below orientation or face-label confusion, including the module front/back direction and case support-ring contact behavior.
- Whether a board or candidate model appears at the wrong scale, position or rotation. Record the asset name and its displayed source bounds.
- Any missing or unexpected finding when module boards overlap the host, each other, or case solids.
- Whether undo, reload, key assembly replacement and circuit removal preserve all unrelated keyboard and module data.
- Any place where the UI appears to imply a measured clearance, completed electrical connection or supported driver that the source evidence does not provide.

## Automated companion

`app/e2e/vik-module-review.spec.ts` opens the same gallery demo, checks the pinned snapshots, model-bound labels, matrix encoder replacements, keymap layer bindings, placement save/Undo/reload, embedded-circuit identity, archive round-trip and blocked firmware export. `app/e2e/vik-case-findings.spec.ts` separately verifies geometry navigation for a module/case collision. Run them after the current app and Rust/WASM bundles have been built:

```sh
pnpm --dir app exec playwright test e2e/vik-module-review.spec.ts
```
