# F7.3 Case viewer private author contract

This is an implementation handoff for the Case vertical slice only. It does
not claim F7.3 acceptance or completion of any future consumer adoption.

## Source boundary and owners

The author keeps `web/src/renderer_host.rs` byte-exact and adds the shared
renderer/view under `web/src/presentation/shared_viewer.rs`. The page-only
`web/src/renderer_host_page.rs` includes that host source unchanged and then
adds `renderer_host_page_extensions.rs` in the same private module. This gives
the binary wrapper private access to the common host lifecycle and a retained
module namespace without adding methods or fields to the library host. The
root coordinator owns page-binary module registration, CasePanel composition,
shared CSS, selection state and persisted display preferences. No public
library API, Rust model, renderer export, dependency, provider or document
schema changes.

## Case projection and parent seam

The Case wrapper consumes the current immutable `Rc<CadScene>` supplied by
`Runtime::cad_scene()`. It uses the existing `captured_case_document` projection
for document-derived data and the matching scene's CAD body meshes, contours,
mechanical stack, Scope and snapshot token. A selected physical-instance scope
is never replaced by the canonical Layout document. A stale scene may remain
visible while generation is pending, but cannot authorize picks or mutations.
The shared viewer's typed generic projection contract accepts Scope, source
token, projection generation and a separate renderer sequence; it does not
require `CadScene`, so a later Parts consumer can supply its isolated sample
projection and advance generation for definition/companion/placement/side/
rotation changes.

The parent supplies selected mechanical layer/finding separately from
Session-owned real-part selection, the resolved light/dark theme as a reactive
input, persisted `CaseDisplay` preferences
(`hidden: Vec<String>`, `colors: BTreeMap<String, String>`), and callbacks for
current mapped picks and display changes. React's preference key is
`boardstudio:case-display:{projectId}:{instanceId-or-boardId}`. The parent owns
the in-memory fallback when browser storage is unavailable. The root resolves
system/light/dark preference and passes the renderer palette; the viewer does
not sample DOM theme state. Camera position, display mode, explosion and section
controls are transient viewer state; hidden IDs and color overrides are
parent-owned persisted preferences.

An empty renderer pick and any unmapped ID are no-ops. In particular, empty
picks do not clear Session selection. Mechanical layer selection is distinct
from real-part selection; renderer IDs never become fabricated domain IDs.

## Owner identity and acceptance

The viewer allocates a unique mount token. Every changed source projection,
including same-Scope/token provisional-to-exact `Rc<CadScene>` replacement,
advances projection generation. Every full/prepared/patch renderer call uses
a third strictly increasing sequence, independent of CAD/provider/document
revisions.

Every callback/output carries full application `Scope`, source snapshot token,
viewer mount token, projection generation, and renderer sequence, plus a weak
reference to a mutable owner stamp. The viewer compares the captured identity
with the active stamp and current Runtime Scope/token immediately before
emitting. The stamp advances with the projection and is invalidated on
unmount, so delayed callbacks can compare against the actual current owner
instead of checking event fields against themselves. The parent must also
re-read Runtime Scope, accepted token, current `Rc<CadScene>` identity and the
current domain mapping before any Session selection/edit action. Display
preference events are similarly owner/scope checked and only update the
parent-owned preference value.

Typed private outputs cover accepted/stale/error scene results, renderer
lifecycle, mapped optional pick ID, optional world point and handle gesture
phases. The current Dioxus Case surface has no editable handle owner; this
slice shows no fabricated handles and submits no edit gesture. Handle export
mapping remains available for a later workflow owner.

## Verified renderer capability map

Existing renderer `Renderer` exports are `setScene`, `setPreparedScene`,
`setPreparedScenePatch` (accepted/stale boolean), `setState`, `setHandles`
(uploaded count), `pick`, `pointOnPlane` (zero or three `f32` values), `fit`,
`view`, `orbit`, `zoom` and `dispose`. The same module exports `prepareScene`,
`decodeStl` and `decodeWrl`. This Case slice maps full-scene acceptance,
display state, handles, picking, point-on-plane, camera and disposal. Optional
typed STL/WRL sources are decoded through the cached module only when supplied;
Case currently supplies none. Prepared-scene and prepared-patch calls are
represented by typed update variants and routed through the same checked
submission path; Case currently submits full scenes only. The page-only wrapper
retains the already imported ES-module namespace after a second cached dynamic
import, without reinitializing the WASM module. Existing public host method
signatures and behavior remain unchanged.

Each renderer scene call consumes a sequence strictly greater than the prior
upload. The baseline host's initial scene uses the immediately preceding
sequence; the wrapper then performs the checked submission and returns the
renderer’s actual boolean. Optional model sources are decoded before that
checked upload. Case supplies no model sources, so model delivery/retry parity
has not been demonstrated.

The renderer host continues to own ResizeObserver/window/DPR listeners,
one-shot frame scheduling, context-loss stop, WebGL cleanup and GPU disposal.
Pointer captures retain their originating full viewer identity. Superseded
captures are released on projection changes, pointer completion, cancellation
and lost capture; stale picks are dropped. Camera orbit and zoom remain
available for retained stale geometry, while pick, layer and persisted display
outputs require the live source guard. Mount work reconciles to a newer
same-scope projection before reporting, and status callbacks use the identity
of the request or last accepted scene instead of reading a newer owner identity.

Viewer callbacks check owner identity; no context-restoration or renderer
retry flow is introduced. Existing Case controls, forms and 2D route remain
owned by the root Case workspace.

## Evidence limits and refactor register

This record is source-backed only. The actual page-binary build, library
checks, paired browser fixture actions, accessibility checks and lifecycle
fault probes remain required at root integration; none is claimed by this
author commit. In particular, the source inclusion and binary-only wrapper
placement still require compiler proof.

RF: no new refactoring takeaway observed. Existing RF-002 and RF-012 remain
the relevant crate-boundary and reflective-host findings.
