# Domain docs

How the engineering skills consume this repo's domain documentation.

## Before exploring, read these

- `CONTEXT.md` at the repo root. It is this repo's glossary; a skill that names `GLOSSARY.md`
  means this file.
- `docs/adr/README.md`, then the ADRs for the area you are about to change.

If a file is missing, proceed silently; `/domain-modeling` creates them as terms and decisions
resolve.

## Layout

Single context: `CONTEXT.md` and `docs/adr/` at the root, no `CONTEXT-MAP.md`.

## Use the glossary's vocabulary

Name a domain concept as `CONTEXT.md` defines it, and not as a term it lists under _Avoid_.

## Flag ADR conflicts

If an output contradicts an ADR, say so explicitly rather than overriding it silently.
