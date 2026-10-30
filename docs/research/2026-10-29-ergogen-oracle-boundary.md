# Ergogen oracle: local boundary assessment

At `0c562aa6d`, the experimental sandbox wrapper's real `--check` succeeded
against the committed fixture. Generation to a private scratch directory also
matched it byte for byte (212,288 bytes); this exercises all 42 authored cases.
The browser reported a WebSocket reset after a clean child exit. These runs used
the pinned checkout and cached locked archives, an isolated Cargo target, and
the pinned Chrome archive. They are not a full gate or independent review.

The boundary is not ready for acceptance. In generation mode
`sandbox_generate.command` bind-mounts the whole destination directory writable
into a namespace where Chrome runs with `--no-sandbox`. With the default output,
that directory also contains the protected legacy oracle. The Python output path
guard therefore does not constrain a compromised browser process to the selected
file. The wrapper also builds the driver before validating the checkout and npm
archives, so it has not established the requested corruption-before-execution
ordering. No weaker sandbox is proposed for acceptance. Keep the existing Node
test in the gate until a corrected boundary is validated and independently reviewed.

The generator's 14 hermetic tests and the runtime preparer's 22 tests passed.
They cover invalid and wrong-HEAD checkouts, protected output paths, atomic-write
cleanup and mocked subprocess failure, but do not establish all of those controls
through the outer sandbox CLI. The real dirty-cache and corruption controls were
not rerun here. The host-probe test ran three offline boundary tests successfully,
then failed setup because `ERGOGEN_CHECKOUT` was not set.

No dependency artifacts were bundled or redistributed in this experiment. The
Git pin identifies the source object and the npm lock SHA-512 hashes authenticate
archives against that lock, not publisher identities. Existing notices cover the
direct packages; a per-package licence and notice review is still required before
distributing a runtime. No fixture or vendor fixture was changed, and no gate
removal is proposed.

## Boundary follow-up (local experiment)

The wrapper now verifies the copied locked archives and checkout before building the driver.
The sandbox mounts a private staging directory writable; the selected destination is checked and
published atomically by the outer process after success. A staged symlink or non-regular file is
rejected. The default destination no longer makes the legacy fixture's directory writable.

A real untracked file and a corrupted cached archive were each rejected by the outer CLI before
an otherwise absent Cargo target was created. An attempted write to the legacy fixture using the
wrapper's namespace command failed with a read-only filesystem. The 16 generator controls, 22
runtime controls and 9 host tests passed inside that namespace with the checkout, cache, driver,
reference and TMPDIR explicitly configured. Those suites were invoked via the namespace command,
not through the outer CLI entry point; this does not close the requested outer-CLI suite coverage.
The host setup failure was an omitted environment variable, not an established host code defect.

No runtime is bundled. No per-package redistribution review is claimed. Chrome still uses
`--no-sandbox` inside the outer namespace; this result does not establish equivalence to all Node
CLI filesystem and Git protections or constitute an independent review. The old Node test remains
required, and the ticket remains open.

## Outer CLI follow-up (local experiment)

From `4d6450b62`, `sandbox_generate.py --check` exited 0 with the pinned checkout,
verified archives and Chrome, and a fresh external Cargo target. It reported the fixture
reproducible. Its log included a WebSocket reset and the driver's clean-shutdown message.
The driver accepts that exact reset only after completed boundaries, a successful Chrome
exit and clean close/drain; other errors remain failures. The host checks the driver's
return code, and the outer wrapper checks the namespace command's return code before
reading or publishing output. A separate mocked outer-wrapper check propagated inner
exit codes 7 and 9, both with and without reset text, and preserved the destination.
Thus this observed reset is teardown noise for this successful run, not a pass criterion.

Direct outer CLI negatives rejected an invalid checkout, the protected legacy path, an
untracked checkout file, and a corrupted locked archive; each exited nonzero before an
otherwise absent isolated Cargo target was created. The unchanged generator hermetic
suite (16 tests) and runtime suite (22 tests) passed as regression checks. They do not
constitute full outer CLI suite coverage: most call Python functions directly, and the
generator CLI helper invokes `generate.py`, not `sandbox_generate.py`. Running the
unchanged suites inside the namespace through a new test-mode wrapper would not test
the outer entry's corresponding behaviors. Full outer-entry coverage remains open.

These checks do not establish full filesystem and Git protection equivalence to the
Node CLI. The trust and stability assumptions above and lack of independent review
remain; no weaker sandbox is proposed for acceptance. The required Node test remains
registered. Logs are local and gitignored in `.evidence/ergogen-outer-*.log`.

## Refined containment rule: rejection

At `research/ergogen-closeout` base `9d8c99c6acc417e83357313b82ce406390eb5647`,
proof 4 failed: a binary copied into the sandbox's private `/tmp` executed successfully.
The following ran locally from the repository root, using `command()` to construct the
production namespace and replacing only the inner payload. It printed
`scratch-execution-succeeded` and `containment probe exit: 0` (the outer Python command
also exited 0):

```sh
python3 -B - <<'PY'
import sys, tempfile, subprocess
from pathlib import Path
sys.path.insert(0, 'scripts/ergogen-parity')
import sandbox_generate as s
with tempfile.TemporaryDirectory(prefix='ergogen-proof-') as d:
    run = Path(d)
    (run / 'output').mkdir()
    (run / 'npm').mkdir()
    cmd = s.command(run, s.ROOT, Path('/usr/bin/true'), Path('/usr/bin/true'), run / 'out', False)
    i = cmd.index('/usr/bin/python3')
    cmd[i:] = ['/bin/sh', '-euc', 'cp /usr/bin/true /tmp/scratch-exec; /tmp/scratch-exec; echo scratch-execution-succeeded']
    result = subprocess.run(cmd)
    print('containment probe exit:', result.returncode, flush=True)
PY
```

The refined rule therefore reinstates REJECT. This probe does not establish a host-visible
write. Privacy, destruction, the other denial checks, parity reruns, and the committed
outer-entry and reset regression suites were not completed. The required Node test and
fixtures remain untouched; step-4 cutover remains blocked.

## Noexec scratch follow-up: rejection

On `research/ergogen-noexec` at `3a6164191`, with the uncommitted noexec
scratch changes and the staging directory bound at `/out`, the real outer
`--check` exited 0 and reproduced the fixture. This does not discharge containment.
The following adversarial probe used the production namespace command and its
noexec mount prologue, replacing only the Python payload. It printed
`exec-exit=0` and `probe-exit= 0`: the sandboxed process could remount `/tmp`
executable and execute a copied binary.

```sh
python3 -B - <<'PY'
import sys, tempfile, subprocess
from pathlib import Path
sys.path.insert(0, 'scripts/ergogen-parity')
import sandbox_generate as s
with tempfile.TemporaryDirectory(prefix='ergogen-proof-') as d:
    p = Path(d)
    (p / 'output').mkdir()
    (p / 'npm').mkdir()
    cmd = s.command(p, s.ROOT, Path('/usr/bin/true'), Path('/usr/bin/true'), p / 'out', False)
    i = cmd.index('/usr/bin/python3')
    cmd[i:] = ['/bin/sh', '-c', 'cp /usr/bin/true /tmp/probe; mount -o remount,exec /tmp; /tmp/probe; printf "exec-exit=%s\\n" "$?"']
    result = subprocess.run(cmd)
    print('probe-exit=', result.returncode)
PY
```

This fails proof 4 of the ratified rule and reinstates REJECT pending redesign.
No host-visible write is claimed. The remaining containment battery and C1/C3/C4
were not completed; the Node gate remains in place and cutover is blocked.

## Follow-up: additional execution surfaces

On the same dirty `research/ergogen-noexec` checkout at `3a6164191`, the remount
probe above was repeated and again printed `exec-exit=0` and `probe-exit= 0`.
Two further probes replaced only the Python payload of the same `command()` and
used fresh temporary directories with `output` and `npm` subdirectories:

```python
# Set cmd[i:] to each payload separately, where i = cmd.index('/usr/bin/python3').
['/bin/sh', '-c', 'cp /usr/bin/true /out/probe; /out/probe; printf "staging-exec=%s\\n" "$?"']
['/usr/bin/python3', '-c', "import os,subprocess; fd=os.memfd_create('probe',0); os.write(fd,open('/usr/bin/true','rb').read()); p=subprocess.run(['/proc/self/fd/'+str(fd)],pass_fds=(fd,)); print('memfd-exit=',p.returncode)"]
```

Both subprocesses exited 0: `staging-exec=0` and `memfd-exit= 0`, respectively.
The first shows that noexec on `/tmp` and `/dev/shm` alone leaves the writable
staging bind executable. The second shows execution from an anonymous writable
file even without remounting `/tmp`. These probes do not demonstrate host-visible
writes. The existing noexec prologue is not a sound hostile-process boundary.

A materially different next experiment is a fail-closed kernel-enforced policy:
make staging noexec too, drop mount capabilities and prevent gaining them through
new user namespaces, then use a tested seccomp policy to reject executable memfd
and other bypasses while retaining Chrome's required JIT and driver operations.
Test the policy against direct execution, remount, namespace, memfd and loader
attacks before rerunning the complete ratified battery and real outer CLI. This
is a proposed experiment, not a demonstrated solution: Chrome compatibility and
complete coverage of executable-code paths remain unproven. C1, C3 and C4 remain
open, no acceptance or cutover is claimed, and the Node gate remains required.

## Capability and seccomp experiment: rejection remains

On dirty `research/ergogen-noexec` at `3a6164191`, a separate experimental
policy used the existing `command()` namespace, remounted `/out` noexec, dropped
all capability bounding, inheritable and ambient sets with `setpriv`, and set
no-new-privileges. A libseccomp filter denied mount APIs, `unshare`, `setns`,
`memfd_create`, and `execveat`; it returned ENOSYS for `clone3` and denied
`clone` with namespace flags. The initial variant returned EPERM for `clone3`
and did not filter `clone`: `clone(CLONE_NEWUSER)` succeeded and Python could
not spawn Chrome. Those results are not evidence for the revised filter.

Reproduce locally from the repository root while the gitignored experiment files
remain available (the scripts contain machine-specific absolute paths):

```sh
python3 -B .evidence/ergogen-policy-experiment/ergogen-policy-run.py
python3 -B .evidence/ergogen-policy-experiment/ergogen-policy-run2.py
python3 -B .evidence/ergogen-policy-experiment/ergogen-policy-run3.py
```

The first run under the revised policy denied direct execution from `/tmp` and
`/out` (126), remount (32), user namespace via `unshare` (1), and direct ELF
loading from both noexec mounts (127). Its memfd probe had a quoting error and
is not evidence. The second run observed `memfd_create` EPERM, and
`clone(CLONE_NEWUSER)` EPERM. Pinned Chrome 156 `--version` and a headless
`data:` page both exited 0; this is a smoke check, not the real driver or parser.
The third run printed `scratch-shell-executed` after writing a shell script to
`/out` and passing it to `sh`. Its Python script probe had a quoting error and
is not evidence. The shell result demonstrates that noexec and this syscall
filter do not enforce the ratified condition that nothing is executed from
writable scratch. All three outer harnesses exited 0; individual probe statuses
above, not that aggregate status, determine the result. The first run's
`unshare` and memfd checks alone cannot establish complete namespace or memfd
coverage; no exhaustive syscall or architecture audit was performed.

This remains REJECT, not a proposed production seccomp policy. A distinct next
experiment is to separate the process that can read writable staging/scratch
from all executable interpreters and the browser/driver process, using a
kernel-enforced read-access boundary (for example, a separately assessed LSM
policy) and a narrowly defined output channel. First prove that shell, Python,
dynamic loader, memfd and namespace attempts cannot consume attacker-written
code, then run the real driver and full battery. This changes process and data
flow and has not been shown feasible or approved as an equivalence claim.
The real Chrome/driver workflow under this filter, private-scratch lifecycle,
remaining battery, C1/C3/C4 and independent review were not run. Node remains
required; no cutover or hosted check was performed.

Prior real-driver evidence is retained above: the 42-case outer `--check` at
`4d6450b62` and the later noexec-scratch outer `--check` both succeeded under
those earlier policies. Neither exercised this capability/seccomp filter; the
headless smoke here does not supersede or invalidate those runs. The shared
Chrome/CDP work in commits `59e9ee753`, `0e8a9e75f` and `93a4c75f7` likewise
is not a compatibility test for this filter.

## Landlock read-separation experiment: blocked

On dirty `research/ergogen-noexec` at `3a6164191`, a new exploratory policy
handled Landlock READ_FILE and READ_DIR, allowing reads only from selected
runtime, checkout, Chrome, driver, proc, device and npm paths, while leaving
scratch writes unhandled. Landlock reported ABI 11. Within the existing namespace,
writing a script to `/tmp` succeeded; `/bin/sh` reading it exited 126 with
permission denied, and Python reading it exited 1 with permission denied. This
is a read-access experiment, not a complete security policy: it did not combine
or validate mount, namespace, memfd, inherited descriptor or output-channel
controls. It does not prove that interpreters cannot consume attacker-controlled
bytes through other channels.

The same policy invoked the real `generate.py --check`, the verified installed
pinned Chrome 156 path and an existing driver binary from the earlier host build.
The generator exited 1 with `maximum recursion depth exceeded`; there is no
receipt that it reached or exercised the driver or Chrome. The driver binary was
not rebuilt or established as matching the present source. Evidence and exact
experimental scripts are `.evidence/ergogen-landlock-19.log`,
`.evidence/ergogen-landlock-run-19.py`, and
`.evidence/ergogen-landlock-probe-19.py` (the runner refers to its `/tmp` copy
of the probe; copy the saved probe there before rerunning). These are new policy
results, distinct from earlier successful Chrome runs. No fixture was published.

This candidate is blocked before the real browser workflow, not proven
fundamentally impossible. The existing generator prepares verified runtime files
under writable temporary storage, so a write-only scratch policy also needs a
separate preparation and immutable handoff design. The next experiment should
identify the recursion failure, prepare and seal runtime inputs outside the
restricted browser process, provide Chrome's necessary profile storage without
allowing it to become an interpreter input, and use a bounded validated output
channel. It must separately test shell/Python, loader, memfd, mounts, namespaces,
file descriptors and publication before claiming containment. The ratified
battery, actual pinned Chrome/driver under this policy, C1/C3/C4 and independent
review remain unrun; REJECT and the Node gate remain in force.

## Landlock traceback follow-up

At the same dirty checkout and HEAD `3a6164191`, the saved Landlock runner was
repeated with its probe copied to `/tmp/ergogen-landlock-probe-19.py` and
instrumented to print the caught exception traceback. Command:
`python3 -B .evidence/ergogen-landlock-run-19.py > .evidence/ergogen-landlock-trace-19.log 2>&1`.
The runner printed `real generator under Landlock exit: 1`. The traceback first
shows `verified_runtime.py:187` raising `Pinned source archive differs from Git
 tree`; while handling that exception, `TemporaryDirectory` cleanup repeatedly
encounters `PermissionError` opening its `/tmp/ergogen-verified-*` directory,
then recurses through Python 3.14 `tempfile._rmtree` and `shutil.rmtree` until
`RecursionError` masks the original error. The policy denies reads of that
extracted directory, including enumeration used to compare it to the Git tree.
This trace does not establish a defect in the pinned source or prove a repaired
handoff is secure. The instrumented probe and trace are local experimental
artifacts; the runner's existing driver is not source-bound.

The next experiment must prepare and authenticate the runtime outside the
restricted process, expose it read-only, and exercise the actual source-bound
driver with a bounded output channel while denying scratch reads and the other
adversarial surfaces. No such handoff or new security acceptance is claimed by
this traceback. C1/C3/C4, privacy/destruction, and the full battery remain open;
the Node gate remains required.

## Prepared host seam (incomplete)

On dirty `research/ergogen-noexec` at `3a6164191`, `host_probe.run` now accepts
an already prepared tree, with authentication and a read-only mount the caller's
responsibility. It is not yet wired into the outer wrapper or protected by a
policy. From `scripts/ergogen-parity`, `python3 -B -m unittest
test_host_probe.HostBoundaryTests.test_prepared_runtime_is_used_without_repreparing`
first failed with an unexpected-keyword `TypeError`; after the change,
`python3 -B -m unittest test_host_probe.HostBoundaryTests` passed 4 tests.
This mock-only check does not establish the runtime handoff or containment.

The shared browser probe uses readable temporary files for driver stdout and
stderr (`tools/browser-harness/test-cad-target-responses.py:46`). The proposed
scratch read denial is expected to prevent that capture; this is a source
observation, not a measured failure under the new policy. A bounded channel
and lifecycle handling remain to be implemented and tested. No new real
42-case run, adversarial battery, privacy/destruction proof, C1/C3/C4, or
independent review was performed. REJECT and the Node gate remain in force.

## Landlock pipe-input follow-up

The exploratory Landlock policy from `ergogen-landlock-probe-19.py` was run
through the existing namespace command with its payload replaced after policy
installation. The payload created a pipe, wrote `echo pipe-interpreter-executed`
into it, and passed its read end as `/bin/sh` standard input. It printed
`Landlock ABI 11`, `pipe-interpreter-executed`, `pipe-exit 0`, and
`namespace-exit 0`. This demonstrates that denying filesystem reads of scratch
alone does not prevent an interpreter from consuming attacker-supplied code via
an inherited pipe. It does not test the proposed combined seccomp policy or
prove that a separately designed process boundary is impossible. A candidate
must account for interpreter invocation and inherited descriptors as well as
scratch paths; the bounded output channel must not become an input channel to
an interpreter. No new integrated implementation or acceptance is claimed.

## Authenticated handoff and bounded staging follow-up

On the same dirty checkout, the outer wrapper now holds the verified source and
locked dependency tree alive and mounts it read-only for the generator. It builds
the driver after verification. The outer reader limits staged output to 4 MiB,
checks it is a regular file opened without following symlinks, validates fixture
structure and canonical encoding, and disables publication pending provenance
and containment proof. Structure validation does not authenticate the output.

From the repository root, the real command
`python3 -B scripts/ergogen-parity/sandbox_generate.py --check --cache /home/chris/.cache/tmp/opencode/ergogen-oracle-19-OeOxaW6C/upstream-checkout --temp-parent /home/chris/.cache/tmp/opencode`
exited 0 and reproduced the 212,288-byte committed fixture using the pinned Chrome
and source-built driver; `.evidence/ergogen-handoff-outer.log` records the reset
and clean shutdown. It ran before the final publication guard, which affects only
non-check mode. From `scripts/ergogen-parity`, `python3 -B test_generate.py
--hermetic` passed 16 tests and `python3 -B -m unittest test_sandbox_output
test_host_probe.HostBoundaryTests test_verified_runtime` passed 30. The new output
test first failed due to missing reader and limit, then passed after implementation.

This is not an integrated containment solution: the sandbox still permits scratch
reads, remounts, and inherited-channel transfers, and the driver capture uses
readable temporary files. No scratch-to-interpreter or scratch-to-output battery,
private-scratch lifecycle, C1/C3/C4 or independent review is claimed. The
read-only handoff is authenticated by its outer caller, not by the hidden generator
argument alone. REJECT and the existing Node gate remain in force.

## Prepared-runtime Landlock integration probe

A fresh experiment at the current shared checkout used the authenticated prepared
runtime and pinned Chrome tree with the existing source-built driver, then applied
Landlock READ_FILE and READ_DIR restrictions before invoking the generator. Run
`python3 -B .evidence/ergogen-integrated-policy-run.py`; the experiment files are
local, gitignored, and use the paths recorded in that script. ABI 11 was reported.
A shell script actually written to `/tmp/scratch-input` failed with permission
 denied (shell exit 126); an attempted read and a scratch-to-`/out` copy both
failed at opening that same file. This is evidence for those specific paths,
not for inherited descriptors, alternate syscalls, mount escape, or output
provenance. The earlier inline pipe result did not establish a scratch read.

The actual generator failed before starting the driver: Python's temporary
 directory discovery reported no usable directory. A second run explicitly set
`tempfile.tempdir` to `/tmp` and reached `Permission denied` on the driver's
capture temporary file. The latest receipt is
`.evidence/ergogen-integrated-policy-run3.log`; the first discovery failure is
`.evidence/ergogen-integrated-policy-run.log`. This probe did not apply the
seccomp experiment, did not reach Chrome, and did not run the 42 cases. It does
not establish an impossibility: driver capture and Chrome profile access still
need a compatible, bounded design and a combined-policy adversarial run. No
publication is authorized by this evidence; the fail-closed guard remains.

## Profile access diagnostic

The source-built driver was rerun under the exploratory Landlock READ_FILE and
READ_DIR policy with `/tmp` additionally readable. The actual pinned Chrome
run reached clean browser shutdown and passed the prior profile cleanup failure,
but the probe also executed a shell script written to `/tmp` (exit 0) and copied
its bytes to `/out`. This policy therefore violates the scratch execution and
publication protections. The generator subsequently failed with `EXDEV` on
`os.replace` from `/out/.ergogen-parity-*/fixture.json` to `/out/fixture.json`;
adding `/out` to the read allowlist did not change that result. Neither run
reproduced the fixture or establishes a 42-case success. Logs and diagnostic
runners are `.evidence/ergogen-profile-readable.log` and
`.evidence/ergogen-profile-output-readable.log` and their corresponding
`*-run.py` and `*-probe.py` files.

The observed conflict is specifically that `remove_dir_all` needs to enumerate
the profile under a directory whose reads are denied, while allowing reads of
all `/tmp` also makes attacker-written scripts readable by an interpreter. A
candidate redesign is to have an outer supervisor create and remove a distinct
profile after confirming Chrome termination, and confine Chrome separately from
the driver with access to that profile while denying it interpreter execution
and uncontrolled input/output channels. The output handoff must also avoid or
explicitly authorize Landlock's cross-directory rename, without making writable
output readable to the untrusted process. This is a design to investigate, not
a proven boundary: Chrome itself can consume code and data, and process-specific
policy, inherited descriptors, provenance and adversarial probes remain needed.
The current integrated policy is experimental and publication remains disabled.

## Same-directory staging correction

`write_fixture` now creates its temporary file in the destination directory and
renames it within that directory. A focused regression test simulated a policy
denying cross-directory rename: it failed with EXDEV before this change and
passed afterward. `python3 -B test_generate.py --hermetic` passed 17 tests.
This does not verify Landlock integration or implement the supervisor, profile,
descriptor, or output trust boundary. Publication remains disabled and the Node
gate remains required.

## Revised trust decision and local rewrite, 2026-10-29

The user trusts the verified source-bound driver and pinned Chrome to follow their
configured workflow. Chrome may read its private profile and scratch; this replaces
only the earlier arbitrary-compromise/no-scratch-execution assumption. Isolation
from host writes and external networking remains required. Output is accepted only
after successful driver and namespace termination, bounded regular-file reading,
fixture validation and canonical encoding, then atomic publication by the outer
process. This is not an assertion that a compromised Chrome cannot execute scratch
or forge valid output. Source and archive identity are relative to the local pins,
not publisher authentication; inputs and temporary parent must remain trusted and
stable during verification. Node remains in the gate pending independent review and
full checks.

On dirty `research/ergogen-noexec` HEAD `3a6164191ada70ef90c45d21866fd3598d5a2921`,
the wrapper uses the authenticated read-only prepared runtime, verified installed
Chrome, source-built driver, private namespace tmpfs profile and bounded pipe capture.
It drops capabilities and sets no-new-privileges before running the generator; it
validates staged output and publishes atomically. The driver owns profile cleanup
on confirmed termination; the private mount is destroyed when the namespace exits.
The controls below are targeted evidence, not a complete kernel security audit.

* `python3 -B scripts/ergogen-parity/sandbox_generate.py --check --cache /home/chris/.cache/tmp/opencode/ergogen-oracle-19-OeOxaW6C/upstream-checkout --temp-parent /home/chris/.cache/tmp/opencode` exited 0; all 42 cases reproduced the committed fixture. `.evidence/ergogen-rewrite-real.log` records the clean-shutdown reset.
* The same outer command without `--check`, with `--output /home/chris/.cache/tmp/opencode/ergogen-rewrite-publish-bMReCP/fixture.json`, exited 0 and `cmp` against the committed fixture exited 0, replacing preexisting scratch bytes. Log: `.evidence/ergogen-rewrite-publish.log`.
* Outer entry and output unit suites passed 10 tests; the success case was RED before publication was enabled. The failure test injects `os.replace` failure and preserves existing bytes; these are mocked inner runs, not real injection through the namespace. Generator hermetic tests passed 17; outer entry, output, host boundary and verified runtime combined passed 36; pipe cleanup passed 4. Initial pipe-cleanup invocation from the wrong working directory failed setup; running from the repository root passed.
* `.evidence/ergogen-rewrite-controls.log` records a production-command payload after capability drop: repository write and remount denied, TCP to 1.1.1.1:53 denied, exit 0. `.evidence/ergogen-rewrite-lifecycle.log` records private marker absent from host `/tmp` during and after clean exit 0, failure exit 7 and killed exit -9. These checks do not prove every host path or egress route, nor inspect every possible residual namespace resource.

The prior outer-entry real invalid/dirty/corrupt controls and reset propagation tests
are recorded above; their earlier receipts are not same-run proof of this rewrite.
C1/C3 have focused mocked coverage, but full real outer negative coverage and C4's
complete ticket receipts remain open. Independent review, complete containment battery,
full gate, hosted checks and cutover were not performed. No fixture or Node gate was
changed.
