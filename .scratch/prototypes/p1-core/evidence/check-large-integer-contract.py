from pathlib import Path
import json
p = Path(__file__).resolve().parent
native = [json.loads(line) for line in (p / "large-integer-oracle-output.jsonl").read_text().splitlines()]
assert all(reply["kind"] == "scene" and reply["document"]["revision"] == 9007199254740993 for reply in native)
runtime = json.loads((p / "large-integer-runtime.json").read_text())["observed"]
assert runtime["outcome"] == "reply", f"Valid native Open must return a provider reply; observed {runtime}"
