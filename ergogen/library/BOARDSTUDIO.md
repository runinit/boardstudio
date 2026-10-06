# BoardStudio footprint assets

Executable footprint generators have moved to the Rust `footprints/` crate.
This directory retains vendored models, reference footprints, source manifests,
licences and attribution. Generator ports preserve source SPDX identifiers and
credits; see `footprints/NOTICE.md`. Historical generator bytes and refresh tooling
remain in Git before the Rust cutover.

Model preparation scripts written in Python remain available. Source and licence
manifests record the pinned upstream assets; generator patch manifests are retired.
