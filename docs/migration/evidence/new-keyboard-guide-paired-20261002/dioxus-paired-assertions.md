# Dioxus paired browser assertions

This is the execution checklist for the Dioxus half of the React oracle packet.
Run it only on root's integrated release candidate after the New guide and
private owner joins are present. Keep the React session `new17-c8a853a97c72`
and its saved project untouched; use a second named isolated browser session.
Do not claim acceptance from source tests or from a partial integration build.

## Fixture and provenance

- Record the integrated Git commit, release build command/result, app URL, and
  browser session ID.
- Use the same viewport/theme as the React oracle. Start from an empty isolated
  Dioxus browser profile, recording that fact without clearing existing user
  storage.
- Create one new keyboard and record project and Main board IDs, accepted
  revision, URL/workspace, localStorage guide preference, and document summary.
- Expected defaults: `Untitled keyboard`; one `Main board` at 1.6 mm; empty
  parts, matrices, and case bodies; default outline/margin and PLA material per
  the React oracle. Project & hardware opens in the Project stage with One,
  Split, and Reversible controls.

## Journey and state transitions

1. Enter `New17 Oracle` in the Project name field and commit with Enter. Confirm
   the accepted document name and that the project/board identity is unchanged.
2. Rename to a transient value and Undo. Confirm the field returns to the
   accepted name. Repeat with surrounding whitespace and commit; confirm the
   accepted value is trimmed. Blur and Enter must each preserve the source
   behavior; blank/unchanged drafts must not create an unintended rename.
3. Continue to layout. The guide stays visible at Layout, workspace is
   Design/Layout, and the real Layout actions are present. Open/cancel the real
   Matrix Setup form and verify guide preference remains open/Layout while the
   form suppresses the guide; cancellation restores Layout. If the integrated
   real matrix owner is available, create one and verify the accepted matrix
   appears selected with the guide restored at Layout.
4. Continue to wiring. The guide remains open at Wiring and workspace becomes
   PCB. Verify source copy/readiness and the real controller/wiring routes;
   do not treat a missing controller-placement capability as a completed
   action or as a disabled fake button.
5. Continue to Case. Verify Case workspace, source copy/readiness, and the real
   Open case settings route. On a 640×900 viewport, verify it reveals Case
   settings and hides Objects as the React oracle does.
6. Continue to Review. Verify Export workspace, source copy/readiness, and the
   real Open export options route. Finish, then reopen Setup guide from the
   Project menu; it must reopen at the saved Review stage and preserve the
   selected project/workspace route according to the source behavior.
7. Exercise Previous step through Review→Case→Wiring→Layout→Project and verify
   stage and workspace transitions. Exercise the Project menu reopen path
   again after a reload to verify the project-scoped persisted stage survives.

## Currentness and ownership

- After every await-capable action, confirm the UI reflects the latest
  accepted document revision and active project identity. Supersede or close
  the active project while a New save is pending if the browser setup permits;
  a late completion must not open the guide for another project.
- Confirm physical setup controls appear only in the Project guide stage and
  submit through the editor-owned typed mount. Switch stage/project during a
  pending action where feasible; stale intent must be rejected without
  mutating the newly active project.
- Confirm guide settings actions reveal the requested inspector/workspace on
  compact and desktop layouts. Record panel visibility and focus separately;
  focus is not inferred from visibility.
- Capture accessibility snapshots and screenshots for New, Project, Layout,
  Wiring, Case, Review, reopened Review, Undo, compact settings, and matrix
  form/cancel/create states when those real owners are integrated.

Record unsupported or unavailable owner joins as blocked acceptance cases.
Do not invent a button, empty editor panel, or successful route to make a case
appear to pass.
