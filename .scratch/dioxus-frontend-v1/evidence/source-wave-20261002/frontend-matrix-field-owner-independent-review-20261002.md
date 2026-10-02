# Matrix field owner repair — independent Spec and Standards review

Reviewed exact commit 3aac941cce1346d07288adf353c0959d187f53aa on 014febc1579edd52da12f0e2fe27c77c30a0e25b in matrix-field-owner-repair-20261002. Production SHA256 df5bbd01ee8a125dc3d81a8598f779b496238394b48b1a1653e6d4b3d8b65cea and harness 678d9da0a9eaf35007a4b48591b0d5d182e8fa2f5b283c2330ef42985ad98dd4 verified.

Spec: approved for bounded source correction. Standards: approved. The five existing owner keys now occur on their own rsx template roots, matching installed dioxus-rsx0.7.10 template_body.rs first-root key extraction. Existing complete owner-key identity stays intact and deliberately excludes accepted token/revision, preserving same-owner typing. The correction changes no public API, member visibility, dependency, operation authority or suppressions.

Independently inspected production import, actual input/blur dispatch and request assertions; reran all four mounted tests successfully. They cover all five fields across same-baseline matrix switch, Matrix/Layout name-target switch, same-owner token/revision advance, and Pending/Saved settlement without duplicate submit. Reused recorded expected original-source red and debug-assertions=no static-template red/green evidence; the latter is meaningful release-branch macro evidence, not a full browser build. Independent test output /tmp/matrix-field-owner-independent-20261002.log.

Root must join New17 7fdccd4a and rerun strict integrated checks to resolve the two unrelated base lint diagnostics recorded by the author. Public release-browser draft/target switching remains open. No numeric/parent closure or RF ledger mutation.
