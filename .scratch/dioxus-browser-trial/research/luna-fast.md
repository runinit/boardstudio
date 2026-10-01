# Luna Fast execution evidence

Verified 2026-10-01 against installed Codex **0.159.3**. Investigation was read-only;
no global configuration, runtime routing, permissions or application code changed.

## Findings and limits

- Installed `codex --help` and `codex exec --help` support model selection with
  `-m`, invocation-only TOML overrides with `-c`, and `--strict-config`.
- The fresh local model catalogue in `/home/chris/.codex/models_cache.json`
  (fetched 2026-10-01T15:26:16Z, client0.159.3) lists `gpt-6-luna`, reasoning
  low/medium/high/xhigh/max, additional speed tier `fast`, and service tier
  `{id: priority, name: Fast}`. The investigator's installed
  `codex debug models --bundled` confirms the reasoning/tier entries.
  A catalogue description of speed is not a measured speedup or guarantee.
- Relevant user configuration in `/home/chris/.codex/config.toml` requests
  `service_tier = "fast"` and Luna/high child defaults. Configured intent alone
  does not prove actual routing. No configuration edit is needed for that request.
- Investigator command
  `codex --strict-config -m gpt-6-luna -c model_reasoning_effort=high -c service_tier=fast doctor --summary --json`
  exited0, with config.load=ok and model=gpt-6-luna. It retained the existing
  rollout/DB inventory warning. This checks recognized invocation configuration;
  doctor did not independently expose effort or the served service tier.
- Root independently read SQLite and stored turn_context for the new research
  thread: **gpt-6-luna / medium** in both. The turn_context has no service-tier
  key. Model/effort are observable; actual Fast service remains unconfirmed by
  this evidence. Future coding must preserve that distinction and report any
  missing runtime confirmation, without silently switching model/provider.

## Session-only invocation

For a new coding session rooted in its newly owned worktree, after its bounded
execution approval:

```sh
codex --strict-config -m gpt-6-luna -c 'model_reasoning_effort="high"' -c 'service_tier="fast"' -C /absolute/path/to/new/owned/worktree
```

For current-session delegation, root requests Luna/high for the single coding
worker and retains itself as coordinator. Fast corresponds to priority service,
not low reasoning. The collaboration interface has no separate tier argument;
record its supported priority routing and the current Fast configuration, then
check observable metadata. Do not claim a served tier if it is absent.
Use Luna/medium for narrow inventory/smoke and Luna/high for coding/review, as
previously directed. No Sol/Astra or external paid provider is selected.

## Sources and capture

Primary sources: installed CLI help/debug/strict-config doctor; narrow catalogue
fields and configuration above; read-only `/home/chris/.codex/state_5.sqlite` and
the referenced stored turn_context. No external documentation was needed.
Requested research model/effort: Luna/medium. Independent route and closure
record: [runtime evidence](luna-fast-runtime.json).
