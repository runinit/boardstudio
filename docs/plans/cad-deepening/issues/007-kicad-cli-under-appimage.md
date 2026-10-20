# 007: kicad-cli tests pass under an AppImage launcher

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)

## What to build

See the kicad-cli decision in [006](006-review-follow-ups-spec.md). The 14 Core tests that run
`kicad-cli` fail when the suite is started from an AppImage-hosted terminal, because
`APPDIR`, `APPIMAGE`, `LD_LIBRARY_PATH` and `GSETTINGS_SCHEMA_DIR` leak into the child.

## Acceptance criteria

- [ ] When `APPIMAGE` or `APPDIR` is set, every test `kicad-cli` invocation drops the four variables; otherwise the environment is untouched.
- [ ] The 14 tests pass both in the leaked environment and with the variables removed.

## Comments
