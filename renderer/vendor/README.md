# cgmath security patch

The renderer retains three-d 0.19.0 and three-d-asset 0.10.0. Both resolve to
the published cgmath 0.18.0 source in this directory through Cargo's patch table.
The upstream Apache-2.0 license and provenance are retained.

`cgmath-security.json` records the verified crates.io archive checksum and hashes
of every vendored file. Compared with that archive, `src/matrix.rs` replaces the
six aliasing pointer swaps with safe value swaps and removes the unused pointer
import. `tests/security_swaps.rs` and a reproducible test Cargo.lock are added. The upstream `.gitignore` is
adjusted to retain that lockfile.

This fixes [RUSTSEC-2026-0197](https://rustsec.org/advisories/RUSTSEC-2026-0197.html).
The same aliasing pattern in matrix element swaps is fixed alongside column
swaps. Equal valid indices are no-ops; invalid indices continue to panic.

The security tests reproduce undefined behavior on the original source under
Miri and pass with the patch. Run them with:

```sh
cp -r renderer/vendor/cgmath-0.18.0 /tmp/cgmath-0.18.0
cargo +nightly-2026-08-12 miri test --manifest-path /tmp/cgmath-0.18.0/Cargo.toml --locked --test security_swaps
```

The copy is needed because the workspace patches crates.io `cgmath` with this
directory, so Cargo treats it as a workspace package in place.

The security audit verifies file hashes and actual Cargo resolution before
allowing the single patched advisory. It restores the registry identity in a
temporary audit input because cargo-audit otherwise skips path dependencies.
Future cgmath advisories still fail the gate. The unmaintained notices for
cgmath, instant and smartstring remain visible and are not security fixes.

Remove this patch when an upstream maintained release fixes these operations
and the renderer can adopt it without an architecture change. Any local patch
change requires reviewing the source diff, rerunning Miri and updating hashes.
