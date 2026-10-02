# PCB06 owner repair — implementation handoff

Base: `71daac1a4db1803a830c8c5f51d545fbee865bbf` (original feature `3384ad66f84c1acedd270fba6413575b850615b0`). This is an implementation report, not independent clearance.

The mounted owner now admits a typed intent with the rendered context/session/document/board/instance/token/revision/generation envelope. Both Case and Project controls retain that envelope while the Dioxus callback updates. An old control cannot adopt the latest accepted source at dispatch. The private `PhysicalSetupMount::submit` constructs the request from its captured owner; no public API is added.

Feedback carries exact operation ID and owner identity; Project and Case each project only their matching result. Busy settlement is operation-qualified. Only this operation's exact accepted proposal may advance feedback attribution to a new token/revision. The exact OutcomeSlot is still registered before Edit and retained until terminal settlement, including after hiding or unmounting; disposed signals are never read or written after unmount.

The fourth argument of private `use_controller` is now `Rc<dyn Fn() -> bool>`, replacing the captured bool. New17 must supply a closure peeking its `Signal<Option<SetupGuidePreferences>>` and checking the same current project ID, open state and Project stage. The private Project-controls Element slot is unchanged. Current standalone Editor deliberately still supplies `Rc::new(|| false)` until that actual join.

## Verification

The native harness compiles and mounts the actual production controller and Project controls. Runtime/model/preparation/timer ports are deterministic; operation observer storage is the actual production registry. It does not claim actual SessionCore or packaged-normalizer/browser coverage. Rerenders explicitly dirty Dioxus APP scope.

`pcb06-lifecycle-red.log`: restored original production controller temporarily, adapting only changed private call signatures in the same tests. Five regression tests fail for the expected stale Edit/feedback/stage reasons; ten prerequisite/observer tests pass. Fixed source restored afterward.

`pcb06-lifecycle-green.log`: 19 pass, including those five plus Case retained source, exact accepted-proposal feedback, hidden-guide observer/feedback restoration without retry, and detached normalization/terminal outcomes after unmount. Token, revision and generation changes are independently exercised.

`native-final.log`: cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page passes 12 library + 37 main + 6 Keycaps lifecycle + 19 physical lifecycle tests. A first broad run could not finish writing its /tmp log because /tmp was full; rerun into this durable directory passed.

Formatting and git diff --check pass. Actual all-target WASM strict Clippy fails only the existing unmounted Project intent variants and `project_setup_controls`; `pcb06-wasm-strict.log` preserves the two diagnostics. No allowance, visibility widening or dummy use was added. Real New17 join and rerun of strict checks remain required.

Controller SHA256: `76526baedb3240fe48d4bf2f5514ea90921de56187fa2c80e842ea0a5b68b12e`.
Tests SHA256: `0c5c8db1127867faac215dcd2c613143ef106ef13bb4b17f08430048ff297010`.
Harness SHA256: `aaf0492fb5e292a3e54c9e77b3f70ae4d7aac1dc23a78b427a9145e58f726049`.

## Open gates and RF handoff

Independent review of this repair is required; this author does not self-clear. Retain RF-001/RF-006/RF-009 attribution for the original owner/lifecycle/evidence findings. No new RF identity is needed. Real guide/Case browser journeys, packaged normalizer, durability/history/reopen, paired presentation and accessibility remain open. No ticket completion or current-demo claim follows from these source tests. Unrelated dirty PCB README and planning files were preserved and excluded from the repair commit.
