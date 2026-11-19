"""Reading Cargo's `--message-format=json` output."""

from __future__ import annotations

from collections.abc import Iterator
import json


def messages(output: str, out, *, flush: bool = False) -> Iterator[dict]:
    """Yield Cargo's JSON messages, printing non-JSON lines and rendered diagnostics to `out`."""
    for line in output.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            message = None
        if not isinstance(message, dict):
            print(line, file=out, flush=flush)
            continue
        if message.get("reason") == "compiler-message":
            if rendered := message["message"].get("rendered"):
                print(rendered, end="", file=out, flush=flush)
            continue
        yield message


def wasm_executable(message: dict, *, kind: str, name: str | None = None,
                    test: bool | None = None) -> str | None:
    """The `.wasm` executable of a `compiler-artifact` message for a matching target, else None."""
    if message.get("reason") != "compiler-artifact":
        return None
    target = message["target"]
    if kind not in target["kind"] or (name is not None and target["name"] != name):
        return None
    if test is not None and message["profile"]["test"] != test:
        return None
    executable = message.get("executable")
    return executable if executable and executable.endswith(".wasm") else None
