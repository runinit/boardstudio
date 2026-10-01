# P3 reference storage compatibility check

This test-only harness executes the existing `app/src/storage.ts` adapter in a
fresh Chromium context on an ephemeral localhost origin. It uses the P3 copied
REVIUNG41 data and public Rust archive entrypoints; it does not touch user storage
or launch the production React session.

The path under test is P3 Rust save/retry and persisted snapshot → Rust archive
→ reference unpack/save/load → reference archive → Rust unpack. It compares the
entire document, all four asset hashes and their Uint8Array representation,
active preference, and rejection of an invalid archive without document changes.
Archive container bytes can differ because the reference writes archive options;
document and asset contents must agree.

With dependencies and generated core assets available, first build P3 with its
own script. From the repository root, run:

```sh
node .scratch/prototypes/p3-reference-check/run.mjs /absolute/path/to/p3-durability unique-attempt
```

Each run keeps a separate ignored target directory. The results record generated
input hashes before and after execution and both checkout commits. Do not treat
an uncommitted worker baseline as an accepted source candidate. Final acceptance
requires reviewed source and its matching artifacts. This is scoped adapter
compatibility evidence, not full React/Dioxus presentation parity, bundled-model
export parity, the full saved-document corpus, or a production single-writer
handoff. The prototype remains disposable and is not automatically adopted.
