# Case contextual gasket support sizing handoff

```yaml
packet_id: case-contextual-gasket-support-sizing-13
stream: Case/shared3D
parents_and_preserved_joins: F7.2 Case contextual workspace; F7.4 mechanical settings; preserve F7.3 and F7.8 joins
private_workspace: /home/chris/.local/share/boardstudio/worktrees/case-contextual-missing-workflow-20261003
private_checkout_proof: git rev-parse --show-toplevel=/home/chris/.local/share/boardstudio/worktrees/case-contextual-missing-workflow-20261003; branch=codex/case-contextual-missing-workflow-20261003; base HEAD=1c200c23c14c15aa34e63f351f5a9f122b4bd149
private_commit_and_paths: source packet commit 746e4e6df64693422b4e5726e6896f00d11eac2f; .scratch/dioxus-case-workspace/{README.md,drafts/13-case-contextual-gasket-support-sizing.md,issues/13-case-contextual-gasket-support-sizing.md}; web/src/presentation/{mechanical_settings.rs,mechanical_settings_controller.rs,mechanical_settings_mount.rs}; this handoff is a follow-up evidence commit
integration_baseline: task baseline 1c200c23c14c15aa34e63f351f5a9f122b4bd149; root checkout observed during handoff at 22b9b06331eaf1505131cc3bfef65dcf81b2ffb2 with dirty tracked diff SHA-256 83ebbaee8f75aca4373058b28d2e915b58dc7c776e772a9c04bb04675963ef8c
mount:
  file_and_symbol: web/src/presentation/mechanical_settings_mount.rs::MechanicalSettingsMount; existing CaseWorkspace Inspector route supplies selected support projection
  contract_hash: existing scope, AcceptedSnapshot and exact support-row identity; callback remains inside existing MechanicalSettings controller request path
  minimal_shared_patch: none; all source changes are private leaf files
styles:
  scoped_selector_fragment: none; existing Mechanical Settings styles are reused
  insertion_point: existing Case Inspector/mechanical settings presentation
checks_and_paired_evidence: rustfmt --edition 2024 on three changed Rust files; git diff --check; leased cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings, exit 0. Agent-browser exploratory comparison used own profile on React5173 pinned 5a472a9426e6e38993361da402cd4ec730feb369 (Sofle v2, Case, Left case assembly; observed Cut length/Pad width support controls) and Dioxus34742 (saved Sofle v2 copy, Case, Left case assembly). No edits were made in either app. Integrated candidate parity journey remains open for root.
independent_review: next consolidated Sol review on the root join candidate; no separate source handoff performed
rf_observation: scoped none; packet records no new refactoring takeaway and retains RF-001/RF-006 history
state: ready-for-integration
root_lease:
  owner: root coordinator
  shared_paths: none changed in root checkout
  start_and_release: private source frozen after leased strict check; root owns integration and candidate verification
build_and_fingerprints:
  mode: affected page-only WASM strict Clippy
  build_id: case-contextual-gasket-support-sizing-20261003
  baseline_provenance_hash: null
  source_pre_and_post: source commit 746e4e6df64693422b4e5726e6896f00d11eac2f; integration checkout was not modified
  reused_asset_comparison: null
  root_and_subpath_manifest_hashes: null
```

The implementation adds the contextual `Gasket N` form to the existing Inspector for exact current generated support rows, with existing request draft/feedback behavior and a support-size patch handled by the existing scoped controller. Linked pair updates retain both anchor identities and metadata; an unlinked support updates only its own dimensions. Selection clearing/acceptance checks include exact support identities. Root owns the integrated browser journey and remaining acceptance joins.
