# WASM-target Clippy coverage

Research snapshot of the working tree on 2026-10-19. No Cargo or build command
was run, so this note makes no claim about existing WASM-target Clippy warnings.

## Current coverage

The shared `lint` step runs `cargo fmt --all --check` and native-default
`cargo clippy --workspace --locked --all-targets --no-deps -- -D warnings`.
There is no `--target` in that Clippy invocation, and `.cargo/config.toml`
does not set a default target. Cargo uses the host target unless a target is
selected, so this Clippy step does not select WASM. [Lint definition](../../scripts/check.py),
[Cargo build target documentation](https://doc.rust-lang.org/cargo/reference/config.html#buildtarget).

CI runs that lint step in its `native` job. The `web` job installs
`wasm32-unknown-unknown`, then runs the checked web build and browser tests; it
does not run Clippy. Those jobs establish compile and test coverage for the
browser code, but do not establish Clippy coverage for it. [CI workflow](../../.github/workflows/check.yaml),
[check step definitions](../../scripts/check.py).

The [pinned toolchain](../../rust-toolchain.toml) already declares both the Clippy
component and the WASM target. Adding the command to the existing lint step should
not require redundant installation or a workflow edit; confirm actual runner
behavior before adding either. No CI execution was performed for this research.

This omission matters for code compiled only on WASM: the Runtime crate gates
its browser `runtime` module on `target_arch = "wasm32"`, while its native
test-support build substitutes `runtime_test_stub.rs`. `runtime.rs` includes
the collision-check module under that browser runtime. Native Clippy therefore
does not inspect that production module. [Runtime crate entry](../../web/crates/runtime/src/lib.rs),
[runtime module declaration](../../web/crates/runtime/src/runtime.rs).

## Narrow command candidate

The narrowest direct check for the code that prompted this finding is:

```sh
cargo clippy --package boardstudio-web-runtime --target wasm32-unknown-unknown \
  --all-targets --locked --no-deps -- -D warnings
```

This selects one workspace package and the WASM target, rather than trying every
workspace package on WASM. The package has one library target and a
`test-support` feature; its browser-specific dependencies are target-scoped.
The runtime source also contains tests gated on both `test` and WASM, including
the collision browser tests. `--all-targets` includes the package's library
and test target configurations, so it is useful if the intended coverage
includes those WASM test modules as well as production library code. Cargo
defines `--all-targets` as lib, bins, tests, benches, and examples; this remains
scoped to the selected package. [Runtime manifest](../../web/crates/runtime/Cargo.toml),
[test module gates](../../web/crates/runtime/src/runtime.rs),
[Cargo target-selection semantics](https://doc.rust-lang.org/cargo/commands/cargo-check.html#target-selection).

For production-code-only coverage, replace `--all-targets` with `--lib`; that
will not lint `#[cfg(test)]` code. The runtime package currently has no default
feature set, and its only declared feature is `test-support`, so do not pass
`--all-features` unless test-support code is intentionally part of the lint
scope. The root web package has a different feature contract: its page build
uses `--no-default-features --features page`, because the default also enables
`core-worker`. Reuse that explicit feature choice if a later scope includes
the `boardstudio-web` page binary. [Runtime manifest](../../web/crates/runtime/Cargo.toml),
[web manifest](../../web/Cargo.toml),
[existing page typecheck](../../scripts/check.py),
[Cargo feature selection](https://doc.rust-lang.org/cargo/reference/features.html#command-line-feature-options).

Clippy accepts Cargo target selection, and Cargo's `--target` selects the
target architecture rather than the host. Dependencies still have to compile
for that target even when dependency diagnostics are excluded. Keeping the
package selection explicit avoids applying this check indiscriminately to
members outside the chosen browser scope, such as `boardstudio-footprints`.
`--no-deps` is consistent with the existing lint policy of not linting the
vendored `cgmath` dependency; the project's own direct package remains linted.
[Clippy usage and workspace options](https://doc.rust-lang.org/clippy/usage.html#workspace),
[Cargo target and package options](https://doc.rust-lang.org/cargo/commands/cargo-check.html),
[workspace membership](../../Cargo.toml), [existing lint policy](../../scripts/check.py).

If the intended product is Clippy coverage for the entire browser application,
the Runtime-only command is deliberately insufficient. Expand to the browser
packages that the existing browser step actually builds (page binary, host,
Runtime, and presentation crates), with each package's required features
declared explicitly. Avoid `--workspace --all-targets --target wasm32-unknown-unknown`
as an unreviewed shortcut: the workspace also contains application, renderer,
footprints, and other packages, while Cargo's `--all-targets`
adds tests, examples, and benches to every selected package. First establish
which of those packages and test configurations are intended. [WASM browser
package command list](../../scripts/check.py), [workspace membership](../../Cargo.toml),
[Cargo target-selection semantics](https://doc.rust-lang.org/cargo/commands/cargo-check.html#target-selection).

## CI integration and warning policy

The existing `lint` step is a natural place to add the focused command so both
local `check.py lint` and the native CI job run it. That makes the target and
feature scope visible alongside current lint policy. The web job already
installs the WASM target, but keeping Clippy in the web job would require
separately invoking or restructuring the named check step; either integration
can be made correct if it avoids racing Cargo against another invocation that
shares the same target directory. Project instructions require a single Cargo
build per target at a time. [Check scheduler](../../scripts/check.py),
[CI jobs](../../.github/workflows/check.yaml), [project build guardrails](../../AGENTS.md).

Continue using `-D warnings`, matching the existing lint step. This makes
warnings from the selected crate fail CI, including Rust compiler warnings;
Clippy's documentation describes this as the CI pattern. Do not add a broad
WASM-only suppression before seeing output. If the first run finds warnings,
classify them from the actual diagnostics and make the smallest correction;
intentional lint exceptions belong in the workspace lint policy with a reason.
This research did not execute the proposed command, so it cannot say whether
the first run will pass or identify any warning. [Clippy warning policy](https://doc.rust-lang.org/clippy/usage.html#command-line),
[workspace lint policy](../../Cargo.toml).

For a temporary lint sensitivity probe, retain the named Clippy diagnostic and
distinguish it from syntax/type errors. The [mutation checker](../../scripts/mutation-check.py)
classifies any output containing `could not compile` as a compile failure. If
Cargo prints that conclusion after denying a lint, preserve the tool's result and
report the diagnostic evidence separately as a lint probe, not as a killed mutation.
This classifier interaction was inspected in source, not reproduced here.

## Acceptance criteria for the ticket

- The standard lint entry point executes at least one explicit
  `wasm32-unknown-unknown` Clippy command for the selected web package scope;
  the command is not only `cargo check` or a browser test.
- The command's package, target kind, target triple, and features are
  explicit, and the selected package's WASM-only source is shown by the command
  to be Clippy-checked. For the collision Runtime minimum, this means the
  `boardstudio-web-runtime` crate with target `wasm32-unknown-unknown`.
- The check uses the existing locked dependency and warnings-denied policy and
  passes in local `check.py lint` and CI.
- The test/build lane and lint lane do not start competing Cargo invocations
  against the same target directory.
- Any required lint exceptions are narrow and justified; no broad warning
  suppression masks WASM diagnostics.

## Sources

- Repository: [lint and target-check commands](../../scripts/check.py),
  [CI workflow](../../.github/workflows/check.yaml), [workspace manifest](../../Cargo.toml),
  [Runtime manifest](../../web/crates/runtime/Cargo.toml), [Runtime cfg selection](../../web/crates/runtime/src/lib.rs),
  [runtime test/module gates](../../web/crates/runtime/src/runtime.rs),
  [web manifest and page features](../../web/Cargo.toml), [project instructions](../../AGENTS.md).
- Official Cargo: [target selection](https://doc.rust-lang.org/cargo/commands/cargo-check.html#target-selection),
  [feature selection](https://doc.rust-lang.org/cargo/reference/features.html#command-line-feature-options),
  [build target configuration](https://doc.rust-lang.org/cargo/reference/config.html#buildtarget).
- Official Clippy: [workspace options and `--no-deps`](https://doc.rust-lang.org/clippy/usage.html#workspace),
  [warnings-as-errors for CI](https://doc.rust-lang.org/clippy/usage.html#command-line).
