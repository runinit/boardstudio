# Case Gasket current-resolution diagnosis

Reviewed integration source `a2bd8b54e6ec6efe7559b22fd6cf752c63cce010`. No production edits were made in the diagnostic isolate `case-current-mechanical-resolution-20261003`.

Initial browser observation: importing the original layered Sofle archive (SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`) in candidate34745, selecting Left PCB/left half, Configure, then Gasket mount briefly showed 0 resolved layers and 0 diagnostics. An initial top-level mechanical-only archive inspection incorrectly suggested that internal Gasket settings had not persisted.

Correction from the exported revision11 archive: the selected `hardware.instances[left].mechanical` and `hardware.sharedConstruction` both contain internalGasket and gasketLayout (autoSize=true, length80, width3, supportCount4). Top-level legacy mechanical is intentionally retained by the established physical-instance update policy. Persistence did not omit the settings.

Completed browser result supersedes the persistent-zero hypothesis. Author evidence commit `32af998e8c9901aaaf01738efb17980d62e32089` retains the reopened candidate snapshot: 17 gasket pairs, 40 layers, exact geometry ready, Gasket1 75×3, Saved. The documented candidate journey resizes70→75, Undo→70, Redo→75 and reload/reopen→75. A follow-up receipt clarification records the transient zero state. React reference used fresh Start Sofle v2; Dioxus imported original5b, so this is strong candidate functional evidence and a reference affordance comparison, not an exact same-archive paired mutation proof.

Source trace:

- Mechanical settings project `cad_jobs::captured_case_document` for the full accepted board/instance Scope. Physical edits persist through `case_settings::update_instance_settings`; canonical mechanical is intentionally separate.
- `Runtime::resolve_mechanical_settings` returns the read-only Core assembly, including blocked assemblies and findings. CAD readiness remains separate.
- `cad_jobs::prepare_captured_case_for_step` returns Blocked before publishing a CadScene when the resolved assembly is generation_blocked or has Error findings.
- `mechanical_settings_mount::project_scene_rows` currently derives current diagnostics and support rows from the current exact CadScene, and retains same-owner older geometry for display. This couples truthful mechanical-resolution display to successful CAD delivery. Retain as an RF-001/RF-006 architectural/current-error-state investigation; the fresh-config browser journey did not prove a persistent blocked-result defect.
- Runtime CAD publication invokes changed(); Editor observes the subscribed version, and CadScene equality is pointer identity. No missed-update source defect was established by the initial transient zero observation. Returning to Case after waiting is not proof that navigation was required for completion.

Do not implement a speculative second cache/controller or alter provider behavior from this diagnosis. If a settled blocked resolver result demonstrably hides its current findings, reuse the existing ResolveMechanical owner and exact accepted identity, separately from the CAD scene. That would be a distinct confirmed fix.

The independently reproduced legacy support admission mismatch was fixed separately in source `3e8aac8e57eb8bcef62ef24f902e1fb7a2b7789f`, regression `e4a89812f0bd7d57c5b44598dbec718f3659240e`, evidence `8726cf1996fe0a4287ac6c13fa9de17be66e5633`: old predicate fails at exact admission error, restored predicate passes1/1. Legacy per-anchor rendered dimensions remain the existing provider limitation; no engine changes or full Case parent acceptance are claimed.
