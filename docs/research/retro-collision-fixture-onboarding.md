# Collision test fixture onboarding review

Date: 2026-10-18. Scope: reduce navigation and fixture reconstruction for future
Runtime collision-check tests. This is a read-only research note; it proposes no
runtime or test behavior changes.

## Finding

The complete Runtime fixture already exists in the collision browser tests, but its
short helper comment does not summarize the ownership prerequisites a new test must
preserve. In the preceding session, the test lane took 18m45s and used 53 command
executions plus 34 file reads while reconstructing test context. Those are session
observations, not a controlled estimate of time saved or evidence about model quality.

The smallest useful improvement is a conditional, colocated test-navigation note
at the top of the collision browser-test module, linked from the existing
architecture paragraph about collision ownership. Point the author to the existing
complete-check fixture and literal operand oracle, and state the readiness and
ownership rules below. Keep the note local to this test area; the architecture already explains
the Runtime/Case ownership split, and the root agent instructions already point to
the architecture and domain vocabulary. A broad agent-workflow rewrite would repeat
existing material.

## Existing fixture and prerequisites

`fixture_document_mode` opens an accepted document, navigates to the main board,
captures its scope and snapshot, prepares a mechanical Case assembly when present,
and installs an exact `CadScene` plus a real `CadWorker` client backed by a controlled
worker. [`mechanical_document`](../../web/crates/runtime/src/runtime_collision_browser_tests.rs#L149)
provides the mechanical configuration; [`fixture_document_mode`](../../web/crates/runtime/src/runtime_collision_browser_tests.rs#L25)
sets up the scene and worker. The controlled worker exercises the message and
qualification shapes while returning `Unknown` proof states; it is not a geometry
qualification provider. See [`COMPLETE_CHECK_WORKER`](../../web/crates/runtime/src/runtime_collision_browser_tests.rs#L1179).

[`complete_check_fixture`](../../web/crates/runtime/src/runtime_collision_browser_tests.rs#L1390)
adds a controlled vendor asset and analysis descriptor, starts a ready worker, and
creates two delivered vendor rows (`switch-1`, `switch-2`) with matching preview
models (`SW1`, `SW2`). [`publish_collision_rows`](../../web/crates/runtime/src/runtime_collision_browser_tests.rs#L1333)
captures a native-preview owner from the accepted snapshot, Case scope, generation,
Core executor epoch and Core identity; begins and publishes the matching lease; and
publishes the model-delivery rows under the same owner. The default row descriptor
lets a complete check bind without importing STEP; the tight-accuracy path replaces
it through the controlled worker.

`CollisionCheckInput` is ready only when its scene is exact and current, the accepted
token and revision still match, the installed `CadScene` is the captured instance,
there is no active Case gesture preview, and captured preview/model rows remain the
currently published values. If that preview has models, rows must exist and have no
pending work. `Runtime::check_collisions` also requires a live view predicate and a
finite nonnegative clearance threshold; it rechecks input and view currency while
the async batch runs. These are enforced by [`CollisionCheckInput`](../../web/crates/runtime/src/collision_check.rs#L11)
and [`Runtime::check_collisions`](../../web/crates/runtime/src/collision_check.rs#L81).

For the current mechanical fixture, the independent expected operand set is the
literal seven IDs `switch-1`, `switch-2`, `plate`, `bottom`, `plate-foam`,
`bottom-foam`, and `pcb-reference`. The expected unordered pair count is 21. The
existing [`assert_complete_unordered_pairs`](../../web/crates/runtime/src/runtime_collision_browser_tests.rs#L1293)
checks the literal operand set, rejects self-pairs and duplicate unordered pairs,
and asserts the exact count. Keep expectations literal: deriving the expected set
from observed findings would let a missing operand disappear from both sides of the
assertion.

## Navigation and documentation fit

The repo already defines Runtime as owning complete collision execution, captured
inputs, currency, readiness and pair traversal; Case owns controls, rendering and
view-lifetime cancellation. The docs also state that controlled-worker tests cover
the complete Runtime check while mounted Case and packaged-page checks cover their
respective layers ([architecture](../architecture.md#L63)). This supports placing
fixture setup details beside the tests instead of duplicating them in the general
architecture page.

`docs/README.md` indexes current references and research, while `docs/agents/domain.md`
directs agents to `GLOSSARY.md` and relevant ADRs. The collision term is already in
the glossary ([GLOSSARY.md](../../GLOSSARY.md#L44)); no new domain term or ADR is
indicated. The recommended landing point is the collision browser-test module's
header note, reachable through one inbound link from the existing architecture
reference. A task handoff may reuse that route, but handoff text alone is not a
durable entrypoint. Start the verification walkthrough from the architecture,
rather than handing the author the new note directly. Use file links and symbol
names; line anchors are not validated by the [Markdown link checker](../../scripts/check-doc-links.py),
which also does not check links inside Rust comments. Manually verify those
references. Do not revive retired workflow documents or add another testing framework.

## Independently verifiable acceptance criteria

- A contributor starting from the existing architecture reference can find the module
  note through its inbound link, then locate the complete fixture,
  the publication helper, the controlled worker and the literal operand oracle
  without searching unrelated Runtime modules.
- The pointer states that preview owner/lease and delivered rows must match, the
  accepted scene and view must remain current, and model delivery must have no
  pending rows before the complete check starts.
- A test using the fixture asserts the exact seven operand IDs and 21 unique
  unordered pairs from a literal expected value, and rejects self-pairs and
  duplicates.
- The helper continues exercising the real Runtime/CadWorker interface with a
  controlled worker; its proof results remain `Unknown`, so these tests are not
  described as provider qualification or packaged-page acceptance.

No build or test was run for this research note.
