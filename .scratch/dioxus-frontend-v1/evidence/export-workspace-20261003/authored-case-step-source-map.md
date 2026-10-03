# Authored Case STEP source map (2026-10-03)

This records source behavior and the isolated Dioxus implementation; it is not
a browser or output receipt.

## Reference behavior

Pinned React source is `5a472a9426e6e38993361da402cd4ec730feb369`.
`app/src/ui/Workbench.tsx` always shows the `case-step` row using
`authoredCaseReady`, and changes only its label to “Authored Case STEP” when a
generated mechanical configuration is active. The same click dispatches
`exportCase`, not `exportMechanical`.

`app/src/exports/cases.ts::exportCase` passes the canonical `context.document`
and `context.scene` to `caseAssembly`. `app/src/caseAssembly.ts` filters saved
`caseBodies` for the selected board and attaches that board's scene contours.
It then calls the existing Core case preparation and CAD export services. The
generated mechanical configuration does not replace those authored bodies.
By contrast, `exportMechanical` explicitly projects the selected physical
instance, resolves the generated assembly, and creates a separate ZIP.

## Dioxus source path

`web/src/presentation/export_workspace.rs` keeps the authored action mounted
alongside the optional generated-mechanical action, including when the row is
labelled “Authored Case STEP”. `Runtime::export_step` guards selected board,
accepted revision, saved bodies, and authored case readiness. Its captured
`step_bytes` path constructs `CaseAssemblyIR` from canonical accepted
`case_bodies`, uses the selected board's canonical contours, calls existing
Core `PrepareCase`, and sends the prepared assembly to the existing exact CAD
worker. Scope, token, document/revision, session epoch, executor epoch, and
Core worker identity are rechecked around asynchronous operations. The
download filename follows React's project/selected-board `-case.step` naming
and uses `model/step`.

The generated package remains a distinct path: it consumes the effective
physical-instance projection and packages the generated stack. The two paths
share selected-context guards but intentionally use different geometry inputs.

## Remaining evidence

No test, build, or browser journey was run for this source packet. The pinned
34763 route receipt observed the authored row disabled for its fixture and is
historical evidence only; it does not verify this new action. A paired
configured-mechanics fixture should confirm both authored STEP and generated
ZIP contents, filenames, and scope-guarded delivery before F8.5 output
qualification. F8.5/F8.6 remain open.
