# Independent Standards: final viewer pointer corrections

Reviewed `66586646...89a5f46bcf160e50fa0088138a8a97b6b6778c65`; shared_viewer blob `af644e88122009bd23644688261a34562f04dbab`. Extensions remain `2173e212b812a338d1bcd407fcff83692bd5fd52`; baseline host remains byte-exact `261ef5bfe1dc984f24be44c95873a684c4940968`.

**Both remaining P2 source findings are closed. No additional material finding in this bounded correction.**

Pointer handlers validate their captured owner identity before changing interaction state. The take helper now matches both pointer ID and full ViewerIdentity, so an old callback cannot remove an interaction that reused its pointer ID under a new projection. Projection-effect cleanup compares against the live owner stamp rather than its captured old projection; capture release follows removal of the matching interaction. This resolves the stale cleanup path under the private contract and `CONSTRAINTS.md` lifecycle rules.

Picking requires current source plus last accepted renderer identity matching the requested projection. Handle movement/end/cancel and pick completion repeat the applied-identity check. Failed/rejected uploads therefore leave old geometry available only for camera navigation, without authorizing a current mapped pick from that old scene. The added regression test directly exercises reused pointer ID ownership, but was not run by this reviewer.

Prior corrected properties remain unchanged: async mount/error output retains request or applied identity, obsolete same-scope mount work reconciles, unmount invalidates the owner before cleanup, and resolved theme is supplied as a prop. Scene sequence tracking and actual acceptance remain in the private wrapper; public host source/API is untouched.

Remaining obligations are integration and proof: root must register the private modules, supply reactive resolved theme and persisted display state, enforce current Runtime/source/domain ID mapping, and mount/style the viewer. Source inclusion, owned reactive arguments, callback/RSX correctness and unused capability variants still need actual strict compilation; this review does not claim those compiler concerns resolved. Full/prepare/prepared/patch/model methods, context loss/disposal, delayed callback faults, stale-scene camera behavior, themes and public accessibility require runtime evidence.

RF-002/012 remain open for binary reachability and reflective capability/lifecycle validation; no new architectural takeaway. No source edits/Cargo/browser execution. Commit whitespace check passed. This is source closure of the reviewed pointer findings, not F7.3 acceptance.
