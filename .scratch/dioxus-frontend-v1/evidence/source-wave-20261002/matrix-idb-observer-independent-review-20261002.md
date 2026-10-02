# Independent IndexedDB observer repair review

Exact repair acdf17718069342b20d6d9438ad8df97b8bba57a against3c0cd7de2dadc6184c6b1771fbe0837a2aa2235a. Read actual production observer and every caller in host/storage.rs, actual browser tests, expected three-red and five-green logs, and strict all-target Clippy receipt. Source SHA storage837bee0d7fcde9371fd0a237bbfeb18b30a528e92210d2dfedb1579cb850208a; tests430ea5ab8dc09ec770fbd76eea53e8ea80cc1ead1de1ea4583616398f99ba662 verified.

Spec: CLEAR for this bounded defect. Completion now tracks transaction complete/abort rather than a possibly handled bubbling request error. Explicit caller failure reason remains first; native DOMException name/message then generic abort provide truthful fallback. Real IndexedDB tests reproduce both dropped callback cases and false-failure-on-success before correction, then verify durable commit, rollback, ConstraintError detail and callback detachment. Historical initial public abort cause remains unknown and packaged regression remains open.

Standards: CLEAR source. Private owning TransactionCallbacks guard is constructed before returning the async future, so unpolled/cancelled future drop clears browser properties before Closure fields drop. Awaited completion also drops the guard; receiver error unwinding likewise owns the guard. No public/member widening, bypass of persistence/session authority or suppressions. Actual callers install one observer per transaction. Reused fresh sufficient author test evidence instead of another cache compile; no self-authorship.

Evidence hygiene note (non-source): commit-range git diff --check flags whitespace emitted by raw browser logs; clean-worktree git diff --check alone does not validate committed evidence. Production source diff is clean. Preserve raw evidence or normalize a copy and state which check passed; do not claim commit-range whitespace clean. This does not invalidate actual red/green results.

RF handoff: proposed RF-014 callback lifetime retained, distinct from accepted scope RF-006. Root owns canonical ledger/integration. No inference that all host observers or MatrixSetup are defective, and no full feature/browser closure.
