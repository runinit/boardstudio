# Domain docs

How the engineering skills consume this repo's domain documentation.

## Before exploring, read these

- `GLOSSARY.md` at the repo root.
- `docs/adr/README.md`, then the ADRs for the area you are about to change.

If a file is missing, proceed silently; `/domain-modeling` creates them as terms and decisions
resolve.

## Layout

Single context: `GLOSSARY.md` and `docs/adr/` at the root, no `GLOSSARY-MAP.md`.

## Use the glossary's vocabulary

Name a domain concept as `GLOSSARY.md` defines it, and not as a term it lists under _Avoid_.

## Flag ADR conflicts

If an output contradicts an ADR, say so explicitly rather than overriding it silently.
