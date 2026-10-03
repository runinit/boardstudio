# Marker07 shared route owner review — d1ca3634

**Standards CLEAR; Spec CLEAR for bounded source**, frozen `d1ca3634b7504244858d166212fa732e59231099`, original docs `64dc70b5b1f764e1f9a571b463813a0afbaf84f2`, corrected receipt-only docs `85e201faff46545c90558536695689ce66b48bde`, clean `/home/chris/.local/share/boardstudio/worktrees/keycaps-navigation-mounted-owner-20261002`. Preserve the original b521 source HOLD report and issue05 owner-review history.

Planning remains the cleared marker child at `1ff36a50aa4f407eff7a0431d128300794d8ee4d`, marker spec SHA `ea3d11ae84fb97255adeffcf09d381a69832300ea7a938bc2ba961735e4bc61c` and issue07 SHA `9791e360959ce728b93ebb061aa96dc07b04248ea328024f31bb367584c4bb61`.

## Standards

The Editor and mounted probe now call the same private focused_finding_for_admitted_route projection after accepted navigation admission and the same use_retire_stale_finding hook. Publication carries the request's accepted full scope/token/revision/finding ID only for an admitted Layout destination. Retirement checks workspace, full scope, snapshot token/revision and active board, reads the focus through peek and clears mismatches without self-subscription. No public API, Core geometry, document/history mutation or second camera/selection owner is introduced. Existing source admission and shared pending-fit/current-destination guards remain in place.

## Spec and production evidence

The added mounted browser probe clicks its actual DOM action, calls production accepted-request admission and route dispatch, updates workspace/selection destination ports, queues the production pending Layout-fit hook, and renders the same production FocusedFindingMarker. It asserts Layout and Outline ownership, the accepted focused finding ID and marker, camera effect emission and fit completion. Leaving Layout through a DOM action exercises the shared retirement hook and clears both retained focus and marker. This closes the previous b521 finding that only an injected marker prop/leaf renderer had been tested.

The underlying marker still selects only the matching accepted SceneDelta finding ID and board and draws Core-provided contours under the existing Layout transform. Earlier b521 renderer tests cover alternate IDs/boards/source replacement and missing markers. This new probe intentionally uses controlled accepted source, geometry and RAF ports; it is not a full Editor, real canvas geometry, physical Core fixture or paired application journey. It does not close selection/Inspector/camera parity findings recorded by the separate c6 public journey.

## Verification and receipt limits

Independently inspected the entire frozen source delta, production call sites, renderer inputs, new mounted assertions and cleared planning; checked clean status, frozen file equality and source-range git diff --check. No heavy check was duplicated.

Author reports a 1/1 Chrome/WASM production-seam test, a disposable publication-disabled expected red failing after the DOM click at `accepted Layout route renders its finding marker`, restored green, strict all-target WASM Clippy, fmt and diff checks. **No raw logs for these d1ca runs were retained; outcomes are in the tool transcript and durable narrative receipt only.** The author confirmed this explicitly. Do not attribute the old b521 raw leaf logs to the new route test. The first absent-button failure was mount timing, not the expected behavior red. Root must qualify fresh combined source after integration; this report does not independently re-certify the tool-transcript command outputs.

Exact source SHA-256:

- presentation.rs `293a30303cf13e41365029d6cbe5c16e735449fa409a3a18a0baaf3237a2e951`.
- keycaps_finding_marker.rs `e10cc940cec1abb2c22af9e0a8465e4ead6fa79f9d2ef5bc8836e2983f8474e3`.
- keycaps_navigation.rs `e5a303d38e18a30e996ce2c8103d04a3e2cd15d89a423190d44f5c2cda2d39f9`.
- Original reviewed narrative receipt `cdc5fb43e90b6fda64b1dcb0a0127bc0ac350026145652a0ecde2a99dba2e27d`; corrected documentation-only receipt at `85e201faff46545c90558536695689ce66b48bde` SHA `59fb8d73f3e354f354c5bafdee8a9fbdf10998503d459695832ea7f2e3b82ee9`, independently hashed/read, explicitly states this raw-log limit.

Bounded source clearance permits serial integration. Keep the real packaged marker/camera/focus/selection/history/save-reopen journey, original missing fixture identity, Case marker ownership and all F6C.4/full-parent joins open. The retained c6 34739 candidate predates Marker07; it records a key-vs-part Inspector mismatch and closer/clipped camera, rather than marker implementation acceptance. No new refactoring finding or RF resolution is claimed.
