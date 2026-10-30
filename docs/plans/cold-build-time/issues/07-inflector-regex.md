# 07: Drop Inflector's regex from the CAD trees

Status: ready-for-agent
Type: build
Blocked by: [02](02-measurement-kit.md)
Parent: [map](../map.md)
Related: [14](14-dioxus-default-features.md), [15](15-cad-test-dev-deps.md)

## Question

Why do all three CAD trees compile regex six times, and can they stop?

## Evidence

- `cargo tree -i regex -e normal,build,dev` in `cad/monstertruck`, `cad/monstertruck-qualification` and
  `cad/wasm` shows one path only: `regex -> Inflector -> step-p21` and `step-p21-derive`. The same is
  true of `regex-automata`, `regex-syntax` and `aho-corasick`.
- `Inflector-0.11.4/Cargo.toml`: `default = ["heavyweight"]`, `heavyweight = ["regex", "lazy_static"]`.
  `step-p21` and `step-p21-derive` depend on it with default features.
- They call only `.to_pascal_case()` and `.to_screaming_snake_case()` (`step-p21-0.1.1/src/ast/de/parameter.rs`,
  `step-p21-derive-0.1.1/src/{type_decl,table_init,select,entity}.rs`). In `Inflector`'s `src/lib.rs`
  those trait methods have no `cfg`, the gated ones come later, and neither case module mentions
  singular, plural, regex or `heavyweight`.
- Cost: 28 units, 129.5 CPU-s, 10.7 s attributed wall (6 builds in each host tree: target and
  proc-macro host, at O3 and O0), plus 7 s of `memchr`.
- Cargo offers no way to turn off a transitive default feature except a `[patch]`.
- The CAD tree already patches its dependencies with vendored copies, listed in `cad_build.py:15`.
  `deny.toml:10`'s licence list has no BSD-2-Clause, Inflector's licence. Whether `check.py deny`
  scans the CAD trees is not checked.

## Gain

| Cold wall | CPU | Warm |
|---:|---:|---|
| -8 to -11 s local, on the CAD lane | -130 to -136 (-4.4%) | none |

CI should gain more, because it is CPU-bound: 130 CPU-s is about 30 s on 4 vCPU, not measured. The
CAD provider wasm should also get smaller, since regex stops being linked into it, if `step-p21`
keeps it today; compare sizes.

Effort: 1 hour. Risk: low. It vendors one more crate of about 2,000 lines with one changed line.

## Work

1. Copy `Inflector-0.11.4` to `cad/monstertruck-qualification/vendor/inflector` with `default = []`
   and a one-line comment saying why.
2. Add `Inflector = { path = ... }` to `[patch.crates-io]` in `cad/monstertruck/Cargo.toml`,
   `cad/monstertruck-qualification/Cargo.toml` and `cad/wasm/Cargo.toml`.
3. Add it to `VENDOR_CRATES` so `wasm_inputs_digest` sees it. Update `cad/scripts/test-cad-build.py` if
   it pins that list.
4. Regenerate the three lockfiles offline, and read the lockfile diff.

CAD review: a dependency slice with no geometry, allowance or API change, so RED/GREEN evidence and a
diff read under the [closure rule](../../cad-backend-replacement/map.md#closure-and-workflow-decisions-2026-10-18-user).
RED is `cargo tree -i regex` finding regex; GREEN is it finding nothing.

## Verification

```sh
for t in cad/monstertruck cad/monstertruck-qualification cad/wasm; do
  (cd $t && cargo tree --locked --offline --workspace -i regex -e normal,build,dev 2>&1 | head -3)
done                                               # each: "did not match any packages"
python3 cad/scripts/test-cad.py > .evidence/inflector-cad.log 2>&1
python3 scripts/check.py deny
python3 cad/scripts/build-cad-wasm.py && ls -l cad/wasm/pkg/*_bg.wasm
python3 scripts/check.py test browser > .evidence/inflector-gate.log 2>&1
python3 scripts/build-time/cold.py inflector test && python3 scripts/build-time/report.py rebaseline inflector
```

## Stop rule

Revert if measured CPU savings are under 60 CPU-s, if any CAD or STEP test changes, or if `deny` needs
a new licence exception you do not want. BSD-2-Clause is a decision: adding it to the allow list is a
small policy change, so raise it rather than slipping it in.
