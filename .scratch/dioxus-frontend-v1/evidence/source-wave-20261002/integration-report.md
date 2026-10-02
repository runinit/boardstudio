# UI wave serial integration — 2026-10-02

Integration worktree `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, branch `codex/rust-v1-ui-parity-20261001`.
Start `bd671ae8db8897388d26ebf973380c69efb9bffd`.
Released HEAD **`dd79af5ba6bc2af304c1412b29e62f2de8b3f8a8`**.

Integrated only the already Spec+Standards-cleared F6 firmware-position editor:
- `32dd07c0a5179fa4c995a1da668365b4e11df2d3` -> `8f237d5f` (source)
- `e4366bdb096f509d18127408f3a0f67f505a6764` -> `dd79af5b` (existing browser evidence)

Both cherry-picks applied cleanly; presentation.rs auto-merge required no manual edits. All four feature leaf files (firmware position choices/presentation, PCB controller, firmware CSS) are byte-identical to the reviewed source. No feature semantics/public API/visibility change, lint suppression or acceptance closure.

Executed at integrated source:
- `cargo fmt --manifest-path web/Cargo.toml -- --check`: PASS
- `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page`: **55 pass** (12 library,37 binary,6 mounted Keycaps harness), `/tmp/frontend-ui-wave-native-integration.log`
- `cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings`: PASS, `/tmp/frontend-ui-wave-wasm-integration.log`
- `git diff --check`: PASS
- Original tracked dirty diff is byte-identical and every original untracked path remains present; snapshot `/tmp/frontend-ui-wave-integration-before.json`.

Source/demo distinction: current source includes F6 at dd79af5b; the latest full build remains root's bd671ae8 `frontend-keycaps-integrated-20261002` served at34720. That demo does not contain the new F6 integration. Existing source-specific browser evidence does not automatically certify this new integrated demo.

Not merged:
- Numeric07 final f0328aee: Standards repaired-source CLEAR in `/tmp/numeric07-standards-source-review-20261002.md`; independent Spec explicitly pending. Do not integrate until its ack.
- Provider/archive f3f58a6a: independent Spec pending; complete source findings/red-green/repair report `/tmp/project-archive-provider-standards-review-20261002.md`.
- Align533f9a50: Standards blocked on two lifecycle defects; `/tmp/layout-align-standards-source-review-20261002.md`.
- Keycaps docs47a02cff/f622c259: inspected overlap and preserved existing untracked F6C.4 findings `spec.md`.47a02cff would add F6C.2 settings at the same path. Author is committing unique `settings-spec.md` and corrected links; replay equivalent distinct final files without overwriting existing F6C.4 spec.

Root owns subsequent ledger/report copies and source/demo-status reconciliation after this explicit checkout release. No further integration write is in progress by this merger.

## Numeric07 subsequent source integration

Root released integration at `e1d66c982a80dbe609c880c76d4fa024da5038e9`. Independent Spec explicitly acknowledged final `f0328aee` after Standards re-review; source series then applied serially:
- `8bc16bc11803266254a1471ee6094d2f7dee5b7d` -> `39b54d38`
- `79e9fe666213b24f40976d953be795305b51f29a` -> `cb80dbfe`
- `232289dbf1a0d82187991ba9c03cb12e41e25c5a` -> `a4a6f85a`
- `f0328aee61b61d19cf37497ee523c5a32882de0c` -> **`ffad6bc4835c7b8b439cdf28663f0624a5e09a94`**.

Clean cherry-picks, no manual conflict resolution or visibility/configuration change. Four controller/inspector/operation/lifecycle leaf sources byte-identical to reviewed final. Existing F6/Keycaps joins retained. Original tracked diff unchanged and all original untracked paths preserved (`/tmp/numeric07-integration-before.json`).

Integrated validation: native **67 pass** (19 library,42 binary,6 mounted harness), `/tmp/numeric07-integrated-native.log`; true WASM all-target strict Clippy PASS `/tmp/numeric07-integrated-wasm.log`; cargo fmt check and git diff check PASS. No additional boxing adjustment was required. These checks do not establish the missing numeric paired/mounted browser acceptance.

Checkout released again at ffad6bc4. Root's served public candidate remains bd671ae8; the current source now includes both F6 and Numeric07 and needs the next source-wave build before demo-current claims. Provider/Case/Align restrictions above remain until their respective reviews/capabilities clear. Root owns further ledger/document commits.

## Provider/archive/build prerequisite integration

After independent provider Spec ack (`/tmp/project-archive-provider-spec-review-20261002.md`), root authorized only the reviewed provider/source series atop ffad6bc4; unrelated Project menu UI was excluded:
- `3039423bbbc6ac9a27de50eef76670a57f4b2eb1` -> `9c1cda3c`
- `fc5d192919249f23e0d78ba02b2d9ef52e6b75e2` -> `1535f726`
- `8587da86e9570604a5131769a826d900e864a56b` -> `1eb19a06`
- `f3f58a6aa6cb2c5b8991535449a5c3ed64980bca` -> `95821328`
- Integration-only duplicate dev-dependency cleanup -> **`9b7008002b69037a4f95f0ab310318e24745b2f3`**.

The first source commit conflicted only in adjacent main.rs module registration: retained existing test-scoped physical_setup and added private portable_archive. Runtime auto-merged; inspected resulting diff, which contains only reviewed archive preparation and private embed preference additions. Reused integration's existing wasm-bindgen-test0.3.79 dependency rather than keep the redundant newly added equivalent cfg table. No runtime feature semantics, source visibility or lint checks changed.

Eight provider/build/test leaf blobs exactly equal reviewed f3f58a6a: hashes `/tmp/provider-integrated-source-hashes.json`. The actual build source inventory contains1251 entries, including all consumed Ergogen generator/library/package and root package/lock/workspace inputs: `/tmp/provider-integrated-build-source-manifest.json`. Preserved original tracked dirty diff and every original untracked path (`/tmp/provider-integration-before.json`).

Integrated checks:
- Native76 pass (19 library,51 binary,6 mounted harness): `/tmp/provider-integrated-native.log`.
- Actual Rust→freshly packaged Ergogen WASM regression1 pass, verifying per-part override: `/tmp/provider-integrated-package.log`.
- True all-target WASM strict Clippy PASS: `/tmp/provider-integrated-wasm.log`.
- Python build-input guard1 + raw-model staging2 PASS (all88 source models/nested paths/aliases).
- Full fmt and diff checks PASS.

Release at9b700800. This is provider02/archive01 prerequisite integration only: F8 UI option, issue16 project-name download filename, and root/subpath/offline paired archive journeys remain open. Existing demo bd671ae8 does not include these sources; next full build is root-owned. Case/Align remain unmerged at their stated open gates. No source writes remain active by this merger.
