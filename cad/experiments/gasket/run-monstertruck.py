"""Bounded native feasibility checks, never interpreted as successful timings."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

binary, fixture, destination = map(lambda p: Path(p).resolve(), sys.argv[1:4])
destination.mkdir(parents=True, exist_ok=False)
reports = []
# 1e-7 is OCCT's usual confusion scale, but Monstertruck's internal tessellator
# rejects values below 1e-6. Coarser 0.01 trials diagnose failures, not acceptance.
for overrun, tolerance in [(0, 1e-7), (0, 1e-6), (0.01, 1e-6), (0, 0.01), (0.01, 0.01)]:
    output = destination / f"overrun-{overrun}-tol-{tolerance}"
    environment = dict(os.environ, MONSTERTRUCK_BOOLEAN_TOLERANCE=str(tolerance),
                       MONSTERTRUCK_CUTTER_OVERRUN=str(overrun), MONSTERTRUCK_DIAGNOSTIC_STEP="1")
    command = [str(binary), str(fixture), str(output)]
    start = time.monotonic()
    report = {"command": command, "overrun": overrun, "tolerance": tolerance,
              "diagnosticStep": True, "timeoutSeconds": 30}
    try:
        result = subprocess.run(command, capture_output=True, text=True, env=environment, timeout=30)
        report.update(exitCode=result.returncode, stdout=result.stdout, stderr=result.stderr)
    except subprocess.TimeoutExpired as error:
        report.update(timedOut=True, stdout=(error.stdout or b"").decode(), stderr=(error.stderr or b"").decode())
    report["wallMs"] = (time.monotonic() - start) * 1000
    reports.append(report)
    (destination / "runs.json").write_text(json.dumps(reports, indent=2) + "\n")
    print(json.dumps(report), flush=True)
