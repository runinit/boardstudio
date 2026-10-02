# Matrix Setup independent repair reack — 2026-10-02

Exact repair `93bd12f885ce80abe687657bd0ecf5aa2bb03ef6` at clean HEAD in `frontend-layout-transform-fields-20261002`; original feature/mount/menu eb230e3e/b479b485/422800d8 and history regression bbfc956c retained. Read issue01, prior independent source findings and durable repair evidence.

**Bounded Spec and Standards repair approval: clear for source integration.** No remaining blocker found in this repair. This does not accept the full Matrix Setup ticket or its Parts placement route.

Source hashes:
- matrix_setup_controller.rs: 3f894400fc2b2388fc4611261e8031677c2b01475c1b8927127f08265de996c9
- matrix_setup_lifecycle.rs: 890e25fa19ccdf3d8e25cda32af760c5155d383a3b847ec230e99b52ff9cfb59
- support/matrix_setup_presentation.rs: 81887a5895bef5bf6a17a4365f1acfb9a2022422170516b01869f6fcd262e5b1

The actual controller admits a physical instance only when it belongs to the captured accepted board; SetMatrix still targets that logical board. It retains existing scope/token/revision/generation/open identity checks before and after preparation. Its Editor-owned alive flag is cleared on drop and tested before post-await signal access, including failures. The admitted detached task holds the exact observed outcome slot until terminal after signals disappear; it does not access those signals during retention. Terminal stale/hidden/absent-owner retirement now happens before interpreting a replacement snapshot's revision or durability. Selection remains contingent on current owner, saved newer accepted matrix and the exact completed operation.

Mounted harness imports the production controller/form and production outcome registry. Runtime/catalogue/timers are explicit deterministic ports, not browser workers or persistence. Read saved red output: original source fails five tests for configured instance admission, lower-revision replacement retirement, absent snapshot retirement, disposed Signal access and dropped outcome observer. Independently reran `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page --test matrix_setup_lifecycle`: all11 pass; log `/tmp/matrix-setup-independent-green-20261002.log`. It additionally checks stale token, invalid instance, hidden settlement and one selection only after saved state. The separate real Session/Core history test and author broad native/WASM checks are reused; no redundant broad rebuild performed.

No production public visibility or schema widening, suppression or alternate accepted authority introduced. Native Dioxus dev dependency is the same pinned0.7.10 needed for the actual mounted regression. Author's extra native all-target dead-code limitation is recorded, not claimed green. Root still must validate the integrated strict target.

Root guide callback/suppression/restoration, focus/cancel behavior, paired public creation/Undo/Redo/save/reopen, all supported presets, menu keyboard semantics and Parts pointer-placement remain open. Carry existing scoped-lifecycle/RF-005 evidence; no new RF identity needed. Reviewer authored no changes in this repair.
