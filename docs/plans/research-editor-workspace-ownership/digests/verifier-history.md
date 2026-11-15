# Independent verification — history

NORMAL fresh verifier; exactly the same10historical commits rechecked plus source at ad9db4730. No PR/incident investigation, working-tree source/status, tests/builds or mutations. Workflow reference read locally as permitted exception only. Refs18-20 verified.

18 verified:1c5de957f Extract private workbench compositions (2026-10-02) moves Keymap SVG/input type while Editor retains assembled fields; explicit Layout inline-pointer exception.9271ade53 Mount accepted PCB scene with guarded selection (2026-10-02) replaces placeholder with typed accepted-state input and PcbScene. Snapshot presentation8269-8295, Keymap179-212, PCB93-130, composition94-105 agree. Commit messages lack explanatory body; no deeper Layout-deferral rationale.

19 verified:1413685e6 Add private Keymap layer edit controller and96874be6b Add private keymap macro editor (both2026-10-02) document Editor-lifetime pending/terminal work. ed92e9d9d Complete panel field bindings and retire departed presentation observations (2026-10-08) adds observed_workspace/None hidden owner/follow_owner before settle. Comments distinguish retained Signals/UI observations; added test source covers departure then Session landing, not executed result. Precision: that commit introduced visibility-reactive changes, NOT every surrounding settlement line. No specific incident/PR motivation established.

20 verified:b29c3abfb Add private PCB wiring preview adapter (2026-10-02) unconditional parent/Editor lifetime query and normalized instance;924ba2820 Qualify PCB resolver error retry and stale board ownership (2026-10-03) injected current-input/resolver closures while production Runtime adapter and checks remain. Snapshot path is web/crates/pcb/src/pcb_wiring/controller.rs (not historical presentation path), ranges934-998/1023-1054/1090-1109 match.

Other exact date/subject checks:784dcb079 Compose editor workspace state handles2026-10-07 parent handler wiring;26f543d1d Migrate workbench and workspace presentation2026-11-05 PCB canvas classes;b100ac59f Migrate keymap workspace layouts2026-11-05 Keymap classes. Dates author-local; committer calendar dates agree.

Complete supplied ranges byte-identical to asserted touch-commit ranges:parent784dcb079, PCB26f543d1d, Keymapb100ac59f, layer/macroed92e9d9d. This establishes attribution AMONG the ten inspected commits, not globally latest touch over uninspected history. All subjects/dates correct with that qualification.
