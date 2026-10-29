# 19: Node-free Ergogen oracle

Status: claimed
Type: task
Blocked by: [Face-completeness replacement](18-face-completeness-replacement.md)
Parent: [CI wall time map](../map.md)
Related: [Web-split design](04-web-split-design.md)

## Question

Can the Ergogen regression retain its authenticated upstream parser and dependencies, real CLI,
filesystem and Git protections, and byte-identical output without executing Node?

## Work and completion

- After the face-completeness decision, test the most direct viable approach using the real pinned
  sources and dependencies. Document their provenance and licensing boundary; fixture-only or mock
  substitutes do not establish equivalence.
- Compare actual outputs and behavior with the existing required test, including negative cases.
  Keep that test in the gate until the replacement has independently demonstrated parity.
- Record a supported approach or a concrete blocker and its evidence before proceeding to the
  remaining migrations. No push, merge or hosted CI.

## Comments

- 2026-10-29: Claimed for the user-directed local implementation experiment on
  `research/chromiumoxide-web-split@0f5d8ae1`; no hosted operations or gate removal.

- 2026-10-29: Ticket 18 is resolved for the local face-replacement decision only. This ticket is
  the next eligible Node-free sequence frontier, unclaimed; the complete browser step and full gate
  are not green and remain in [22](22-node-free-gate-verification.md). Proceeding here follows the
  user-approved face-local parity then Ergogen sequence, not a claim of full Node independence.

### Local original-parser prototype — 2026-10-29

At `research/chromiumoxide-web-split@0f5d8ae188752065622ed2cd7e3b367fde9640f2`, a private,
throwaway Chromium 156 CommonJS loader initialized the actual pinned Ergogen `points.js` and
`units.js` and ran all 42 authored `cases.cjs` configurations (360 output points). Its serialized
output was **byte-identical** to a separate local Node reference: 14,136 bytes. Both were passed to
the repository's actual `collectExpected` and `validateFixture` functions and assembled using the
generator's fixture shape; each matched the committed 212,288-byte fixture exactly. This is not a
run of the real generator CLI and the assembly/check used Node outside the gate. The custom loader
does not implement full Node resolution, isolation or failure semantics and is not a candidate to
ship as-is; Chromium used `--no-sandbox` and file access only for this private experiment.

Provenance: `git archive 8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37` read locally available Git
objects; a separately initialized private checkout fetched that exact object from the local repo
over `file://`, had the pinned HEAD and clean status. This authenticates against the repository's
existing commit pin and Git object identity, not an independent signature or remote-origin proof.
`prepare.py` read each of the 20 locked npm archives directly from the local content-addressed npm
cache, verified its SHA-512 against `scripts/ergogen-parity/package-lock.json`, then extracted it
to a private directory and checked package versions; no arbitrary installed tree or download was
used. That establishes archive-to-lock integrity, not independent publisher authentication.
The pin's Ergogen LICENSE is MIT; Maker.js and Math.js declare Apache-2.0; the lockfile declares
the other packages MIT except `kdbush` ISC. Archive source, licenses and notices are **not**
redistributed by this experiment. Any future bundled or vendored runtime needs its own complete
per-package license/notice and provenance review, including generated bundle attribution; the
fixture provenance is not a substitute. See `scripts/ergogen-parity/THIRD_PARTY_NOTICES.md`.

Local evidence and reproducible commands (paths are machine-local and may be ephemeral):
`/home/chris/.cache/tmp/opencode/ergogen-oracle-19-OeOxaW6C/`, outside `.evidence` and all
checkouts. Commands used from that directory are below; they are a record, not safe to rerun
over these outputs. For a repeat, use a fresh owned directory, update the hardcoded root in
`prepare.py`, and use new output and browser-profile paths:

```sh
python3 prepare.py                         # already run once: 20 verified archives + Git archive
node reference.cjs                         # local Node reference, not migrated gate
chromium --headless --no-sandbox --disable-gpu --disable-background-networking \
  --allow-file-access-from-files --user-data-dir="$PWD/chrome-profile-4" \
  --dump-dom "file://$PWD/probe.html" > browser-dom4.txt 2> browser-log4.txt
node compare.cjs reference.json
node compare.cjs browser-result.json
node compare.cjs browser-corrupt.json      # expected failure
node negative.cjs                         # Node parser rejects null points
```

The first Chromium attempt (`browser-dom.txt`) failed because the exploratory loader treated a
file-directory listing as JavaScript; the second (`browser-dom2.txt`) failed on a missing-file
XHR. After correcting those *prototype loader* errors, `browser-dom3.txt` passed. A Python base64
extraction of its `result` element into `browser-result.json` compared equal byte-for-byte to
`reference.json`. `browser-dom4.txt` records the same parser and a negative null-points input;
both browser and Node reported `Error: Field "points" should be of type object!`. Altering the
first actual point by one made `node compare.cjs browser-corrupt.json` exit 1 with `Generated
fixture differs`. A missing-parser copy (`probe-missing.html`, `browser-missing.txt`) reported
`ERROR: ... Cannot resolve`, and flipping one byte in the Maker.js archive in memory made its
SHA-512 fail the lock integrity comparison without modifying the cache.

The existing required test was separately run with Node as a local reference, **not** as a
replacement gate: `node scripts/test-ergogen-parity.cjs
/home/chris/.cache/tmp/opencode/ergogen-oracle-19-OeOxaW6C/upstream-checkout`. It passed its
11 offline groups plus the pinned-checkout untracked-file negative control: invalid checkout and
wrong HEAD rejected before output, protected legacy output unchanged, unrelated temporary file
preserved, and dirty pinned checkout rejected. This does not prove an eventual Node-free CLI
preserves those behaviors. Existing gate registration and source are unchanged.

Next concrete experiment: build a minimal Node-free host/CLI around a separately provenance-reviewed
browser loader, using a fresh owned external pinned checkout and verified archives. Exercise the
real CLI `--check`, output generation and byte comparison for all 42 cases, then reproduce the
existing invalid/wrong-HEAD/dirty-cache, protected-path, atomic-write and subprocess failure
controls against that host. Verify archive/source corruption fails before executing or writing,
and prove full test-name and behavior parity before replacing the required test. Preserve normal
browser sandboxing and fail closed on browser/module errors; do not import the older broad CDP
suppression. No full gate, Node-free process-tree audit, packaged distribution/license review,
or independent review has been performed. The ticket is released for the next agent, not resolved.

- 2026-10-29: The user's later prioritization supersedes the earlier sequencing instruction above:
  this ticket is not a technical prerequisite for the seven CAD ports. It remains open and is a
  separate prerequisite for [full-gate verification](22-node-free-gate-verification.md).

### Input preparation and one-case host experiment — 2026-10-29

The following is uncommitted experimental work in
`/home/chris/.local/share/opencode/worktree/3ca6f9/chromiumoxide-web-split-experiment`,
at `research/chromiumoxide-web-split@3d816250c5e5658f74d19800a83904c12f0ca517`.
It does not replace the existing Node Ergogen gate. Evidence paths below are relative to that
checkout, gitignored and potentially ephemeral.

`scripts/ergogen-parity/verified_runtime.py` prepares the pinned source and locked local npm
archives. Its bounded range handling accepts canonical exact versions, caret ranges and `*` only;
it checks direct and transitive dependency closure, archive name/version/metadata and integrity,
and rejects nested `node_modules`, archive links and invalid paths. It rejects Git filter
configuration before checking status, validates the no-optional-lock index, and checks the
temporary parent against protected paths before probing it with `tempfile`. This is validation of
the reviewed current lock, not general npm support: unsupported features fail rather than being
approximated. The local pin and lock establish identity against those local inputs, not publisher
authentication. The temporary parent must be trusted and the checkout and lockfile stable during
preparation; concurrent modification is not excluded.

- `.evidence/ergogen19-safety-recheck-green.log`: 22 tests passed;
  `.evidence/ergogen19-safety-recheck-real.log` records preparation using the actual locally pinned
  checkout and cached archives.
- `.evidence/ergogen19-semver-reference.log`: 2,496 canonical release/range comparisons against a
  local Node semver reference, with zero mismatches; Node was used outside the gate only.
  `.evidence/ergogen19-lock-mutations.log`: three range mutants killed.

The experimental `host_probe.py`, `host_probe.mjs` and `test_host_probe.py` use the real verified
prepared source and archives and the pinned parser for **one case only**. An independent host
review found no isolation guarantee in the earlier run (host networking and unsandboxed Chrome),
loose successful-JSON validation, an untrusted driver and failed modules remaining cached. The
parent subsequently tightened the one-case output to an exact finite schema with boolean and mode
validation, rejected nonfinite JSON and evicted failed modules from the cache. The red and green
boundary logs (`.evidence/ergogen-host-boundary-red.log` and
`.evidence/ergogen-host-boundary-green.log`) record the initial two failing tests and their fix;
`.evidence/ergogen-host-boundary-browser.log` records seven passing tests, including cache retry
and the actual parser. That browser run used `bubblewrap --unshare-net`, masked Node and a clear
environment, but exposed the read-only host filesystem and ran Chrome with `--no-sandbox`.
`.evidence/ergogen-host-cache-mutation.log` records the cached-failed-module mutant killed.

The driver and browser paths remain operator-supplied and are not authenticated against current
source; the sandbox remains inadequate for untrusted inputs. These experiments make no broad
isolation claim and the limited host review is not a final review of a replacement. This one-case
host has not run all 42 cases or the real `--check`, nor established generation, atomic filesystem
and protected-path controls. No hosted run, full gate, Node independence or timing proof follows.
The existing Node Ergogen gate stays in place; this ticket remains unresolved and
[22](22-node-free-gate-verification.md) remains blocked on it.

## Comments

- Claimed 2026-10-29 for terra-xhigh implementation delegate
  (step 2: all 42 cases + negative controls + sandbox assessment; branch
  `research/ergogen-oracle` from `0c562aa6d`, no PR; fixture regen and weaker-sandbox calls
  are user decisions, not the delegate's).
- Step 2 partial 2026-10-29: parity proven (real sandboxed `--check`, 42/42, 212,288 bytes
  identical; 14 generator-hermetic + 22 runtime-prep tests green) on `research/ergogen-oracle`
  (`7a976b1f`, pushed, no PR). NOT met: sandbox verdict (Chrome can write outside scratch,
  driver builds before checkout/archive validation), dirty-cache + corruption controls through
  the outer CLI, host-probe `ERGOGEN_CHECKOUT` setup. Findings in
  `docs/research/2026-10-29-ergogen-oracle-boundary.md`. Remainder delegated same day
  (terra xhigh, branch `research/ergogen-boundary` from `7a976b1f`): confine + validate-first
  ordering, outer-CLI verification, setup fix, licence review only if bundling.
- Boundary hardened 2026-10-29 on `research/ergogen-boundary` (`4d6450b6`, pushed, no PR):
  private staging dir, atomic publish, validate-before-build; dirty-checkout + corrupt-archive
  rejected pre-build, protected-fixture write fails read-only, 16 generator + 22 runtime +
  9 host-probe green. Gap: full suites ran via namespace command, not the outer CLI entry —
  follow-up delegated same day (terra xhigh, branch `research/ergogen-outer-cli` from
  `4d6450b6`). WebSocket reset after clean exit under investigation (exit codes decide).
  No runtime bundled, no fixture changed; sandbox not yet accepted as equivalent.
