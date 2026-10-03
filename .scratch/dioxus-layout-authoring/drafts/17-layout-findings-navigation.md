# F3.5b Layout findings footer and live navigation

**Parent:** F3.5 — Layout inspector, relationships and findings.

**What to build:** Add the real Layout findings footer action and its contextual Inspector page. Present the current accepted scene findings using the pinned React grouping, labels, severity and target-action semantics. An action selects or shows the live target, fits its accepted geometry/marker and opens the relevant Inspector context.

**Start condition:** The production Layout mount already exposes an accepted `Scope`, snapshot token/revision, scene findings, owner generation, tree selection callback, board-navigation callback and camera fitting helpers. This child starts from those callable seams; it does not wait for full F3.1 or F3.5 acceptance.

**Observed React contract:** `Workbench.tsx` renders `Layout findings: N` from `presentedFindings(scene.findings, document)`. The action opens/pins Inspector, closes Objects, and shows a `Layout findings` page with findings grouped under their resolved target label. `FindingList.tsx` shows severity and message, then “Show outline” for an outline target or “Select affected geometry” for a resolvable non-outline target; unresolved targets have no action. `showFinding` retains an action while navigating to another board and resumes it after that board is accepted. On same-board navigation, it routes Outline to the existing outline Inspector, Part to component selection, Matrix to matrix selection, and Board to board context; it fits target geometry and accepted finding-marker contours and focuses the Inspector.

## Contract

- The footer count and page are derived from the currently displayed accepted Layout scene and accepted document. Use the established `presentedFindings` / `groupedFindings` behavior, including duplicate feature-wrapper collapse, target union, severity ordering, first-resolved-target grouping and the `Layout review` fallback.
- Capture source `Scope`, accepted snapshot token, document revision, Layout workspace and current selection-adapter generation in each navigation request. Revalidate that owner and the exact finding against the currently accepted scene before acting. A stale, superseded or unmounted request has no effect.
- Resolve targets with the existing precedence: active-version outline feature, document outline feature, part, matrix, body, then board. For Layout, offer navigation only for a target that resolves to a live board/object in the accepted document. Preserve target labels and exact action labels; never render a dead button. Layout does not claim mechanical-layer or Case-body routing.
- If a valid target is on another board in the same accepted document, retain the original request and navigate using the current scope and existing board-navigation callback. Resume only after the same session/document/token/revision is still accepted and the expected target board is active. Re-resolve the target and its owning board, then perform the same guarded route. A changed token/revision/document or a different destination retires the request.
- Same-board Part actions use the existing component context/tree selection path; Matrix actions use its existing matrix context; Outline actions use the existing Outline tree context and Inspector lifecycle; Board actions use its existing Board context. Keep selection and camera mutation within existing session callbacks/events.
- Fit using the existing Layout target bounds and camera path; when the accepted scene has finding-marker contours for this finding and board, include those contours using the current marker bounds behavior. Do not reconstruct diagnostic geometry from message text or target IDs.
- Opening the footer action reveals/pins Inspector, closes Objects, and opens the dedicated `Layout findings` page. Back and Escape leave the page and restore keyboard focus to the footer trigger. Opening focuses the page heading. Target actions close the findings page, reveal/pin Inspector, and focus its first actionable control after route settlement.
- Keep current contextual component, matrix, outline, board, Geometry scripts and other workspace Inspector owners intact. The page is transient presentation state over existing accepted document/scene and selection; no findings store, new domain operation, public API/type/member visibility or renderer is introduced.

## Acceptance

- [ ] In Layout 2D and 3D, footer `Layout findings: N` matches the accepted scene's deduplicated React count. Activating it opens/pins Inspector, closes Objects, and shows the `Layout findings` page with grouped headings, severity, messages and exact action labels.
- [ ] Same-board part, matrix, outline and board actions select/show the corresponding live target, update the appropriate existing Inspector context, fit its accepted geometry and any associated marker, and focus Inspector. An absent/stale target remains readable with no action button.
- [ ] A cross-board target navigates to the owning board and resumes the same action only if the original accepted token/revision/document/session remain current after board selection; stale or superseded pending requests do nothing.
- [ ] Back and Escape return to the prior contextual Inspector without changing accepted document/history and restore focus to the footer action; opening the page focuses its heading.
- [ ] Compare one grouped multi-target Layout scene and one board-switch finding against pinned React. Preserve existing target and history semantics; this UI slice issues no document edit.
- [ ] Reuse `keycaps_fit` target/group/marker-bounds helpers only as general presentation helpers; do not create a Layout `KeycapsFitSource` or route Layout findings through Keycaps fit state. Keep full F3.5 and all other parent criteria open.
- [ ] Preserve RF-001–015 and record any new scoped refactoring observation, or explicitly state none observed.

**Out of scope:** Changing finding production, severity, geometry, diagnostic messages, or deduplication policy; Mechanical/Case finding routes; PCB/Keymap/Keycaps finding consumers; new selection/camera authorities; selection editing, outline version editing, or full F3.5/F3.7 acceptance.

**Source inventory:** `app/src/ui/findings.ts`, `app/src/ui/FindingList.tsx`, `app/src/ui/Workbench.tsx` (`showFinding`, Inspector page, and footer); `web/src/presentation/keycaps_fit.rs` (existing finding presentation/target/marker bounds); `web/src/presentation/objects/tree.rs` (accepted target contexts); `web/src/presentation.rs` (`LayoutOwnerIdentity`, `select_tree`, `navigate`, camera owner); `web/src/presentation/layout_workspace.rs` and `canvas_status_footer.rs` (private presentation mounts).

**RF handoff:** Preserve the shared migration register; no new refactoring takeaway observed for this bounded UI composition unless implementation reveals a source-backed structural issue.
