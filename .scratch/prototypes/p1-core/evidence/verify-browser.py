from pathlib import Path
import json, subprocess, socket, time, sys
p = Path(__file__).resolve().parent
prefix = sys.argv[1]
name = "boardstudio-p1-core-20261001-" + ("subpath" if prefix != "/" else "root")
sock = socket.socket(); sock.bind(("127.0.0.1", 0)); port = sock.getsockname()[1]; sock.close()
log = (p / (name + "-server.log")).open("w")
server = subprocess.Popen(["python3", "-m", "http.server", str(port), "--bind", "127.0.0.1", "--directory", str(p / "site")], stdout=log, stderr=subprocess.STDOUT)
commands = []
def browser(args):
    command = ["agent-browser", "--session", name, "--executable-path", "/usr/bin/chromium", "--json", *args]
    result = subprocess.run(command, capture_output=True, text=True, timeout=25)
    commands.append({"command": command, "exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr})
    return result
verdict = {"status": "failed", "url": f"http://127.0.0.1:{port}{prefix}"}
try:
    time.sleep(0.5)
    assert server.poll() is None, "task-owned server did not start"
    assert browser(["open", verdict["url"]]).returncode == 0, "browser open failed"
    result = None
    for attempt in range(25):
        output = browser(["eval", "(() => { const text = document.querySelector('#report')?.textContent; try { return JSON.parse(text); } catch { return null; } })()"])
        assert output.returncode == 0, "DOM evaluation failed"
        result = json.loads(output.stdout).get("data", {}).get("result")
        if isinstance(result, dict): break
        time.sleep(1)
    verdict["report"] = result
    browser(["snapshot", "-i"])
    errors = browser(["errors"])
    assert errors.returncode == 0, "browser error inspection failed"
    assert json.loads(errors.stdout).get("data", {}).get("errors") == [], "unexpected page errors"
    assert isinstance(result, dict), "Rust report did not complete"
    assert result["status"] == "passed", result
    assert result["prefix"] == prefix, "host prefix mismatch"
    assert result["open"] == "passed" and result["snapshot"] == "passed"
    assert result["invalid_stale_unsolicited_rejected"] == 3
    assert result["sender_bytes_after_transfer"] == 0 and result["received_bytes"] == [3, 1, 4]
    assert all(result[key] == "settled" for key in ["close", "crash", "init_failure"])
    verdict["status"] = "passed"
except Exception as error:
    verdict["error"] = str(error)
finally:
    browser(["close"])
    server.terminate(); server.wait(timeout=10); log.close()
    verdict["commands"] = commands
    (p / (name + "-browser.json")).write_text(json.dumps(verdict, indent=2) + "\n")
print(json.dumps({k:v for k,v in verdict.items() if k != "commands"}, indent=2))
sys.exit(0 if verdict["status"] == "passed" else 1)
