# Case generated-mode parity diagnosis

**Cause: different selected physical scopes, not a synthesized configuration from an empty mechanical document.** No source changes or Cargo commands were made. This uses the diagnosing-bugs workflow with the verifier’s existing public paired reproduction; duplicate browser execution is delegated to that verifier. CodeGraph was queried first in the indexed original checkout; conclusions below were checked against pinned React 5a472a9426e6e38993361da402cd4ec730feb369.

## Evidence and hypotheses

Public candidate dc81237c and reference captures are under `/var/tmp/frontend-run/case-keymap-followup/`. `candidate-case-baseline-snapshot.txt` explicitly shows Physical instance = Canonical board, Add case settings and New case body. `reference-case-baseline-snapshot.txt` shows Main case assembly, its generated layers and mechanical controls; `reference-case-visible-text.json` supplies the generated note.

I decoded both JSON string envelopes in `case-{candidate,reference}-initial-doc.json`, extracted `docs[0]`, and compared complete objects: **equal**. Both are document edbec892-9952-4b6b-a7ba-cb6b0a514ac4, revision 3, 85 parts, zero authored bodies, canonical mechanical null. Crucially, both have `hardware.instances[0] = {id: "main", boardId: "main", mechanical: <saved configuration>}` and populated sharedConstruction. The saved configuration is a printed gasket stack. Comparing only top-level mechanical lost the relevant source channel.

Ranked hypotheses were (1) differing selected instance, (2) absent-config defaults synthesis, (3) unsaved reference preview. Source and stored data confirm (1), reject (2) for this path, and make (3) unnecessary. A public candidate selection of `main` is the discriminating control; verifier is checking it on the existing profile. That control is not a fresh-build green or a waiver for initial selection.

## Exact source chain

- React `useProjectSession.ts:72` clears explicit selectedInstanceId on open.
- React `main.tsx:36-37` resolves an explicitly selected instance belonging to the selected board, otherwise the **first instance belonging to that board**. It does not require that instance to have mechanical configuration.
- `useCaseGeneration.ts:94` passes that selected instance to `effectiveCaseDocument`.
- `hardwareInstances.ts:13-24` overlays its saved configuration/shared construction and applies mechanical defaults to an existing configuration. With **no instance and no canonical mechanical**, line 14 returns the original document. Defaults do not create a stack from nothing here.
- `Workbench.tsx:94` supplies physicalCaseDocument to `useCaseWorkspace`; its `generatedCase` compares that effective mechanical board with the selected board, and hides authored forms while retaining/counting canonical bodies.
- Candidate `application/src/session.rs:1358` resets active_instance_id to None on open. `web/src/cad_jobs.rs:485,512-520` interprets None as canonical and uses document.mechanical, correctly null for this scope. The controller’s generated_stack check consequently evaluates false. `case_settings::initial_settings` creates a UI settings draft but does not determine generated_stack or mutate the accepted document.

Thus the existing F7.2 condition is correct for its supplied scope. Our earlier source contract/review did not trace the upstream initial-selection fallback far enough to qualify end-to-end parity.

## Coordinator choice and minimal correction boundary

**Recommended:** keep the authored/generated predicate and same-scope F7.2 acceptance unchanged; record the public initial-selection red under F7.7/F5 scope ownership. Implement the missing selection policy privately in the root frontend using existing guarded navigation: retain a valid selected instance for the board; otherwise choose the first matching instance in document order; use None only when no matching instance exists. Resolve this on project-open/board-context transitions before enabling Case actions. Reuse scope/generation checks, drag cancellation and selected-context/anchor cleanup, and submit existing Event::Navigate. Rendering, settings callbacks, generation and outcome ownership must then consume the resulting real Session Scope.

Do not fabricate a projected instance solely inside CaseBodyInspector, treat any populated instance as globally active, synthesize mechanical defaults, or write SetMechanical/ReplaceDocument to achieve a display change. Those approaches misalign display and mutation/job identities.

The existing Dioxus explicit Canonical board option differs from React’s fallback when a board has instances. Full policy integration must explicitly reconcile that option for Case and board navigation, including configured-board navigation, rather than repeatedly overriding an available user choice after render. F7.7 owns that UI decision; this does not authorize a Core/API/schema change or silently amend the current F7.2 navigation contract.

A one-time guarded Case-entry selection would be a smaller initial-Reviung correction, but **partial**: it does not establish React parity for later board changes, invalidated instance IDs, reopen or the Canonical option. Do not mark full initial/ongoing selection parity accepted from that patch.

## Required checks and refactoring evidence

Preserve original import→Case red, then rerun against the fix without manually normalizing scope. Also compare same-scope `main` controls; boards with no instances; first matching instance without its own mechanical; explicit second/flipped instance; board switch/reopen; delayed drafts/results; unchanged canonical document/revision/history. Keep full generated viewer/configuration/generation gates separate.

RF-006 receives concrete canonical-versus-physical scope evidence. RF-009 receives the source-accounting gap: identical ProjectDoc and a local generated predicate do not imply identical selected Scope. Add these observations to the existing handoff/register through its owner; no new broad abstraction or refactor is proposed here.
