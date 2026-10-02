# Standards review: active project and physical instance navigation (final formatting identity)

Reviewed candidate: `e1e8606ec04ba49ff8846b88b4d2c7aab9dbe003` (focused build source commit `b50ddbdd6b74a2489e80078e4a0d2923aeca6edb` plus integration/evidence merge `019ac939970708616dbcae071ea56dde719db36e`). Base: `37cc0580fc9a3e7ab569cad1ff1bbc47de1ad7b0`.

Production diff SHA-256 (`web/src/runtime.rs` and `web/src/presentation.rs`, base to final): `4306da0608a168b0165ea4a3124873777bb3df32a35f191ec94f7aa2b8fbad58`.

Session regression test diff SHA-256 (`application/tests/durable_session.rs`, `8b00777fd499aeafe660ad0cb7f11eef97d7bbb3` to final): `3bcbf23e9d18759cf39faf932944ae3bf8e737bcccbd1a46b66d184f6c1c5340`.

Combined changed-file diff SHA-256 (all three paths, base to final): `7895289585011358589b7a2efdf40a68948a9bbb1fc3651ddc58bf111d81fdb7`.

The final `b50ddbdd` commit changes formatting only: multiline formatting in `durable_session.rs` and a compact `Runtime::report` match arm. The production behavior is unchanged from the previously reviewed implementation. That implementation persists the active project ID in the existing scoped preference, restores through the existing storage/core worker flow, and uses the session's scoped navigation API for canonical/instance transitions. The selector is derived from current-board instances and includes a canonical option; validation and cancellation remain in the application session. No API visibility, storage schema, worker protocol, or persistent data format changed.

The added session test exercises valid physical-instance navigation followed by canonical scope, cancellation of queued generation/export work, and rejection of invalid board/instance IDs while preserving scope. It asserts emitted public effects and read-model scope rather than reproducing private helpers.

Findings: none. Provider reported checks passed on this candidate: application 14, web 12, formatting, strict native/WASM checks, and focused Dioxus build. Browser verification is recorded separately against the immutable focused artifact; that artifact omits service-worker/offline policy, so it cannot establish offline restoration.
