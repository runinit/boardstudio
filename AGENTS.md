# BoardStudio

- Read [architecture](docs/architecture.md) when locating ownership across Rust and browser providers; [CONTEXT.md](CONTEXT.md) defines domain terms.
- Use the commands in [README.md](README.md) for development and checks. Browser presentation is compiled only for WASM; native tests alone do not validate it.
- Before changing components, footprints, modules or models, read [component onboarding](docs/hardware/component-onboarding.md).
- Preserve unrelated work and user project data. Stage explicit paths. Keep fixes and their verification within the affected behavior.

## Agent skills

### Issue tracker

Specs, wayfinder maps and tickets are committed Markdown under `docs/plans/<effort>/`. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context: `CONTEXT.md` at the root and ADRs in `docs/adr/`.
