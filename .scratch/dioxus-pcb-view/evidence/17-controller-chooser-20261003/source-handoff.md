# Ordinary PCB controller chooser source handoff

Add controller is now mounted in the no-controller Wiring context. The chooser captures the accepted snapshot, scope and generation, opens Parts filtered to controller, enables the existing controller placement action, and exposes Back to PCB. Browsing and Back do not edit the design.

A separate private PcbController workflow uses the existing definition loader, pending canvas part, accepted ReplaceDocument transaction, terminal observation and history. It returns to PCB after commit or cancel. Setup-guide placements retain their guide-stage admission. Current owner predicates suppress old snapshots, sessions, boards, generations and delayed definition results. The chooser retires on leaving Parts or replacing its owner; active placement then uses its own existing owner.

Affected strict page WASM all-target Clippy passed; formatting and diff checks passed. The final ordinary-pane visibility Boolean is covered by the forthcoming package compile. Changed paired journey, full parent acceptance and consolidated candidate review remain pending.

RF-001 takeaway: the existing controller entry handler depended on guide preferences while its callback shape described no origin. Missing UI could not safely be wired without a concrete caller origin and explicit return context. This packet introduces that narrow ordinary origin; broader shared composition/intent-state cleanup remains deferred. No public API or project format change.
