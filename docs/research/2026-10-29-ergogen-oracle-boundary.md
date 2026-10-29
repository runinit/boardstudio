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
