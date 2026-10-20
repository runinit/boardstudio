# 003: Kernel tests build through the production case builder

Status: ready-for-agent
Type: build
Blocked by: None
Parent: [CAD deepening](../map.md)

## What to build

Production builds every case through the kernel's case builder over a prepared case.
The kernel also exports a second profile builder that only tests use, so export, profile
validation and cache-complexity tests verify an interface production never calls. Move
those tests onto the case builder with a prepared plate, and make the profile builder
internal so the kernel has one public solid builder.

## Acceptance criteria

- [ ] The profile builder and its input type are no longer public kernel exports.
- [ ] Export, profile-validation and cache-complexity tests build through the case builder.
- [ ] Native test count is unchanged apart from deliberate renames recorded here.
- [ ] Native kernel tests and `check.py test` pass.
