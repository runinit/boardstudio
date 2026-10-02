# Mechanical generation observation — no confirmed regression

Candidate `http://127.0.0.1:34675/`, frozen source `0cad7577`; original verifier session `mechanical-ui-layout-0cad7577`, real REVIUNG41 copy configured Gasket. Root authorized read-only use of the verifier session; verifier paused commands and supplied provenance. No source/provider changes, navigation, cancellation, storage writes, process termination or session replacement occurred.

Verifier's observations: Generate clicked around07:30 local; Cancel remained enabled and no resolved preview at roughly3s and13s; a subsequent read command queued/blocked approximately another10s. Its later completed snapshot showed Cancel disabled,58 resolved layers,88 diagnostics and the interactive3D image. No precise click/completion timestamp pair was captured, so total CAD duration cannot be reported as an exact number. The short early window is not a bounded performance contract.

Independent read-only command:
`python3 /tmp/frontend-run/mechanical-generation-latency/check-settled.py mechanical-ui-layout-0cad7577`
passed: snapshot command0.0494s, Cancel disabled and interactive preview present. Full snapshot and timestamped JSON are retained. The initial local assertion incorrectly searched literal `[disabled]` while agent-browser emitted `[disabled, ref=e54]`; this harness-only parsing failure was corrected to bracket-token matching. It was never an application red result.

Classification: this run completed generation. Temporary generation latency and queued browser-command behavior were observed; a permanent stuck-generation regression is falsified for the observed run. We have neither a measured equivalent-input previous-build latency control nor evidence locating the delay specifically in CAD versus driver/main-thread work, so no performance regression or exact cause is claimed. No red-capable actual bug reproduction was obtained, and the diagnosing-bugs workflow stops before speculative hypotheses/fixes. Further performance work would need a timed same-document/same-settings comparison on current and prior provider builds.

Current session remains intact for the verifier. Read-only console/page-error outputs are retained separately. No production change is justified by this observation.
