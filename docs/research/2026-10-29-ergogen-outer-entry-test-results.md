# Ergogen outer-entry test results, 2026-10-29

Worktree HEAD: `3a6164191ada70ef90c45d21866fd3598d5a2921`, with uncommitted
experimental changes. These results do not close ticket 19 or establish sandbox acceptance.

`python -m unittest discover -s scripts/ergogen-parity -p test_sandbox_entry.py -v`
exited 1: five tests passed and `test_success_publishes_valid_fixture` failed (expected 0,
got 1). This suite calls `main()` in process and mocks Chrome verification, archive copying,
the driver build and the sandbox subprocess. Wrong HEAD, tracked dirt and untracked-file
checks use a real temporary Git repository; the latter two use a patched pin. The staged
symlink/FIFO, check match/mismatch, publication guard, and exit 7/9 with and without reset
text exercise the outer function with a mocked inner process. The publication success
assertion is intentionally left failing while the current security guard disables publication;
atomic success and failure preservation remain unverified.

Separate real subprocess invocations of `python3 -B scripts/ergogen-parity/sandbox_generate.py`
used an external temporary checkout, output, temporary parent and npm cache. A freshly
initialized wrong-HEAD checkout exited 1 with `Cache is not the pinned Ergogen commit`.
A local clone of pinned `8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37` exited 1 for
each of tracked dirt and an untracked file, with `Pinned checkout has tracked modifications
or untracked files`. With a clean checkout and a copied locked cache whose
`@babel/runtime` archive was replaced by corrupt bytes, it exited 1 with
`Archive integrity mismatch: node_modules/@babel/runtime`. Each invocation preserved the
existing output bytes. These subprocess controls used actual Chrome verification and archive
handling; they did not instrument driver execution or test the inner sandbox. The temporary
clone and cache were removed after the checks, without changing the source checkout or cache.

Real CLI staged-output injection, atomic publication and reset propagation remain unverified.
The security guard remains in place; no successful publish or C2 claim follows from these results.

## Functional follow-up

The earlier publication guard and failing success test described above have been
superseded by the rewrite recorded in the oracle-boundary research document. On
the current dirty checkout, `python3 -B scripts/ergogen-parity/test_generate.py
--hermetic` passed 17 tests; unittest discovery for `test_sandbox*.py` passed 10,
and discovery for `test_verified_runtime.py` passed 22. These include mocked
publication success and failure preservation. The recorded real outer check and
replacement publication both reproduced the 42-case fixture. The Node gate remains
until independent review and the full checks; these results are functional evidence,
not security certification.

Unqualified `test_*.py` discovery ran 56 tests but reported two setup errors:
the opt-in upstream test classes require `ERGOGEN_CHECKOUT` and other runtime
environment. This invocation is not a full-gate result.
