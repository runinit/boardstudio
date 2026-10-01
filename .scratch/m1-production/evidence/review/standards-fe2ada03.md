# Standards review — `fe2ada03`

Compared `2040e23b...fe2ada03` for `scripts/build-m1.py`; this is the only production-file change. The patch moves the prior Dioxus public directory into the unique build output before each prefix build.

## Actionable hard breaches

None found. The build output directory is created with `exist_ok=False`, and the preserved directory is placed under that build's output. This avoids deleting prior build material and gives each `dx build` a clean public directory, preventing old hashed WASM and cache inventory from being copied into the staged site. The following staging and source-hash logic is unchanged. `git diff --check 2040e23b fe2ada03 -- scripts/build-m1.py` passed.

## Carried judgment-call smells

- `scripts/build-m1.py:62` names the preserved directory `previous-dx-public-{mode}`. On the `subpath` iteration, the directory being moved is the freshly generated root build output, so the suffix describes the build about to run rather than the moved directory. This is a minor evidence-label ambiguity; it does not affect the move or staged assets.

No source mutation or build/check was performed for this review. The active untracked release inventory and `scripts/__pycache__/` were left untouched.
