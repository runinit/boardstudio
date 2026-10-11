# 141 — Diagnose mirrored outline gap controls

Status: claimed
Type: task
Blocked by: none

## Report and scope

The user reports two sets of gaps in the Inspector and needing to uncheck both a left and right gap before they are ignored. Establish the actual control identities, linked-outline ownership and generated effect before changing shared behavior. Whether the report involves one outline spanning both halves or separate outlines is awaiting clarification; this does not block observational reproduction.

L3/E owns diagnosis and any subsequently confirmed Inspector/adapter fix; coordinate a Core reconciliation defect with L1/B [108](108-reconcile-mirrored-outlines-assemblies.md). Preserve independent per-outline settings unless the linked contract proves they should propagate. Do not collapse controls merely because they look similar. Unchecking protection permits outline repair and does not necessarily remove support geometry.

## Acceptance

- Reproduce with an independently specified minimal synthetic linked pair; no demo board fixtures.
- Identify each displayed control by board, outline feature and gap identity, including correspondence between halves.
- Exercise the real Inspector edit/resolver route, one side at a time from fresh baselines. Record protection state and generated contour effects on both halves.
- Distinguish duplicate rendering, lost linked propagation, legitimate independent settings and geometry-generation behavior with actual evidence.
- If a defect is confirmed, retain an expected-failing regression before the smallest coherent fix, then affected native and actual mounted WASM checks plus one fresh Sol6.1 medium review. A product ownership question is escalated with the concrete scenario; do not invent coupling.

## Execution authority

E may prepare a minimal observation/regression fixture in `web/crates/layout/src/outline_lifecycle_tests.rs` and `web/crates/layout/src/outline_lifecycle_browser_tests.rs`. No Core edits or API/schema expansion are granted by this ticket. Report any different required paths before editing. Source-bound impact and complete change-analysis gates apply. Runtime checks await a GLOBAL HEAVY grant with four build jobs, four native test threads and one suite at a time; no full lane suite.

## Evidence so far

L3 preparation: `/home/chris/.cache/boardstudio-baselines/mirrored-outline-gaps/repro-preparation.json`, binding eight source hashes and existing native/mounted seams. Inspector rows follow selected board projection gaps and edit individual gap IDs; creation mirrors gap identity/points and `keep_gaps`, but this does not prove ongoing propagation. Focused native own-ID regression passed1/1 and actual mounted Chrome observation passed1/1, including final-source execution. Durable record: `/home/chris/.cache/boardstudio-baselines/mirrored-outline-gaps/141-observation-summary.md`. Each one-sided uncheck changed its own setting and the source contour changed locally. However, the fixture independently protected the live right candidate after creation: the inherited protected ID differed from that generated candidate, which also had different member provenance. This does not establish corresponding gaps or reproduce the reported linked failure. The same writer is checking literal reflection and a symmetric fixture before any causal claim or product fix. Only the browser observation test changed; no Core/API changes. E explicitly released HEAVY; B107 correction owns it.

Follow-up literal comparison by the same diagnosis writer attributes the first candidate mismatch to asymmetric fixture membership: the inherited protection points reflect correctly, but the right candidate includes an extra generated matrix cell excluded by the source envelope. This is fixture evidence, not candidate-ID instability. A symmetric minimal fixture is being prepared; its actual runtime observation remains pending behind107's HEAVY window.

The follow-up separated geometry from identity: source points `(14,20),(14,5),(10,5),(10,10)` reflect about x100 to `(86,20),(86,5),(90,5),(90,10)`. The reflected member/local-point identity hashes differently from the suffix retained by `mirror_gap`; this alone is not a product failure because protection may match by region. The same writer prepared a symmetric fixture before creation, removing unrelated matrix membership and using direct electrical mode, without reseeding right protection. Browser-only source SHA256 `88869d553253af48311ac8d8c87bc2f1c64d4eae4ba252c15603d38bb4614a18` is uncompiled/unrun. The exact same-filter mounted Chrome observation with `--nocapture` is queued after B107 explicitly releases HEAVY. No fresh review or production fix follows solely from this preparation.

Durable follow-up source/fixture binding: `/home/chris/.cache/boardstudio-baselines/mirrored-outline-gaps/141-source-classification-symmetric.json`, SHA256 `4cf7e4cc9ae51fa52a623ad25f159567b8fbea8eec242df5d03f5880227d6db5`. Contains exact source hashes, reflected points, identity algorithm and queued command/observations. This is source classification; runtime remains pending.

### Symmetric runtime observation

Same writer completed actual mounted Chrome1/1 PASS (166 filtered), with no post-creation right seed, then explicitly released HEAVY to107. Browser source SHA256 `eec389f8538cddf7c461e9e00d7d11f4c8290611db7437343ee073f33331539d`; raw log `/home/chris/.cache/boardstudio-baselines/mirrored-outline-gaps/141-symmetric-run.log`. Immutable two-observation artifact `141-symmetric-observation-captured.json` in the same directory has SHA256 `4bca5eb4fdb9e51e897abcc4479a3c8a884a9c0562de5115af9e65e688386133`. The writer updated the earlier classification filename; its original source-only content is preserved as `141-source-classification-before-runtime.json` with the original4cf7e4cc hash.

Both actual controls initially recognize inherited protection. From fresh baselines, unchecking main removes main protection while right stays protected, and the reverse edit removes right protection while main stays protected; contours are captured for both. This establishes independent ongoing settings at the preactivation baseline, without a product fix. It does not yet prove exact generated-gap correspondence: the source protected candidate span10.77/44 contour vertices differs from right span10.296/51. The same writer is qualifying reflected geometry against captured points/contours read-only. Carry valid linked-edit behavior into108/111 and browser acceptance; do not fix IDs or collapse controls from ID differences alone.

### Captured geometry qualification

`/home/chris/.cache/boardstudio-baselines/mirrored-outline-gaps/141-geometry-qualification.md` SHA256 `163e39a6711e4e7daa86c5d4a8283636caea45ec625f3c8fb38c2a6ae49dcac7` and sibling JSON SHA256 `e8c874cb0561289275fbd65d65139450767256b05e74f413b025ab9a03a25710` reject exact baseline contour reflection: reflected-source→right maximum point-to-segment distance5.09, reverse5.9444, tolerance0.002 for rounded capture. Source protected candidate has4points/span10.77; right protected has3points/span10.296, also matching another unprotected source candidate's span. This confirms association with inherited protection, not correct reflected recess identity. Cause is unknown from captured contours alone. Same writer has a bounded source-only causal trace of fixture/profile/frame/envelope/protection inputs; no production change or new runtime is granted. Pre-edit creation mismatch cannot be attributed solely to later unactivated edit reconciliation.

### Bounded causal trace and next Core observation

E's same-writer source-only trace is complete/STOPPED: `/home/chris/.cache/boardstudio-baselines/mirrored-outline-gaps/141-causal-trace.md` SHA256 `e2a677b8dd88a3edece697d5941cafa90fa3dbadc113c36437a400b7c0e05bf5`, sibling JSON SHA256 `6c48170d7bf7dcf48cbc731b28cd85077a9dd2edf3470a35e901a1ccbd06f8bc`. Symmetric source and target use the same front square courtyard; part poses, selected membership and reflected control frames are consistent under inspected creation rules. No remaining fixture asymmetry or post-creation reseed was found. The earliest observed divergence is the pre-edit generated contour. It persists in captured side-unprotected outputs (`keepGaps=[]`,32vertices each), bidirectional reflected distance maximum6.3813/mean1.2606; copied protection alone is insufficient to explain it. Specific generator-stage cause remains unproved.

Next smallest diagnostic, assigned to L1/B's dependency-ready108Core work after107: capture immediately after typed creation and before edits BOTHboards' selected member IDs, resolved world courtyard rings and chosen automatic bridge endpoints before cleanup. Compare courtyard inputs first to separate creation/projection divergence from bridge/cleanup divergence. E has no Core write lease; no production/API change, broad geometry rewrite or extra generic audit is authorized by this finding. Preserve the existing observation test/evidence.
