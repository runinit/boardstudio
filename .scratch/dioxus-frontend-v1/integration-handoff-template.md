# Private stream integration handoff

Use one record per bounded ready packet in its existing stream evidence folder. The root coordinator owns the shared integration queue and lease; authors submit this record and continue their next private packet. This is queue metadata, not a new canonical task graph or automated lock.

```yaml
packet_id: <existing child ID / packet ID>
stream: <Layout | PCB | Keymap | Keycaps | Case/shared3D | Parts+Project>
parents_and_preserved_joins: <canonical IDs and all applicable acceptance joins>
private_workspace: <absolute isolated author worktree>
private_commit_and_paths: <frozen commit + exact private paths; dirty hashes if any>
integration_baseline: <root HEAD + dirty-diff hash>
mount:
  file_and_symbol: <root-owned existing mount location>
  contract_hash: <reviewed input/callback/scope contract>
  minimal_shared_patch: <attached diff; root applies serially>
styles:
  scoped_selector_fragment: <attached private fragment; no global selector drift>
  insertion_point: <root-owned CSS file + section>
checks_and_paired_evidence: <commands/results + fixture/actions/React/candidate IDs>
independent_review: <reviewer ID/profile + contract/Standards/Spec scope/verdicts>
rf_observation: <existing RF update evidence or scoped no-new-takeaway note>
state: ready-for-integration
root_lease:
  owner: <root coordinator>
  shared_paths: <exact files consumed by this integration>
  start_and_release: <source freeze through final hash comparison>
build_and_fingerprints:
  mode: <full | independently reviewed page-only reuse>
  build_id: <unique ID>
  baseline_provenance_hash: <full-provider baseline; null for full>
  source_pre_and_post: <identity/hash comparison>
  reused_asset_comparison: <exact asset hashes; null for full>
  root_and_subpath_manifest_hashes: <fresh route manifests>
```

Root advances the queue through `ready-for-integration`, `integrated-source`, `packaging`, `wired`, `paired-verification`, and `accepted-packet`, or records the exact `blocked` gate. These states do not accept a canonical parent. A compiled leaf can remain unmounted; a wired leaf can remain browser-red. Independent integrated Standards and Spec review and applicable paired browser gates remain required before packet acceptance.

The root lease covers only declared shared paths and the source freeze for that candidate. Other stream authors work in their isolated private lanes. A changed callback owner, selection/transaction identity or lifecycle contract receives focused independent Sol 6.1 High review before dependent implementation. Semantic integration repairs retain their failing reproduction and are reviewed independently after correction. Public API/schema/cutover approval boundaries stay unchanged.

Evidence: [reviewed six-leaf contract](../dioxus-workbench-parity/evidence/workbench-composition-contract-v3-reviewed.md), [current routing](AGENT-ROUTING.md), [provider reuse proposal](specs/page-only-packaging-reuse.md). RF-001 and RF-009 retain the source/integration and provenance limitations; this template adds no checker or enforcement claim.
