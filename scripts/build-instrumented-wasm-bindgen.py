#!/usr/bin/env python3
"""Build a logging-only instrumented wasm-bindgen pair for the launch-wait experiment.

Experiment only (ticket 1.2, finding-launch-wait.md section 7): rebuilds the exact
checksum-pinned wasm-bindgen-cli 0.2.129 and tiny_http 0.12.0 crates with logging
edits only. The tiny_http TaskPool queue condition stays byte-identical to stock
(asserted before and after the edits); the patched pair from
build-patched-wasm-bindgen.py is neither built nor selected by this script.
Instrumentation is inert unless BOARDSTUDIO_LAUNCH_LOG_DIR is set at runner runtime.

Example (use a spacious job-local directory, not a shared Cargo target):
  python3 scripts/build-instrumented-wasm-bindgen.py --output ~/.cache/my-job/wasm-bindgen
  BOARDSTUDIO_WASM_BINDGEN_DIR=~/.cache/my-job/wasm-bindgen BOARDSTUDIO_LAUNCH_LOG_DIR=... \
    python3 scripts/check.py build browser

Use --offline to require cached crates and dependencies. --crate-dir can point at
previously downloaded .crate archives; otherwise Cargo's registry cache is searched.
--sources-out DIR additionally copies the instrumented crate sources to a new
directory (outside .evidence) so they can be diffed against stock. The temporary
source and target live alongside the output and are removed on failure.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import urllib.request

VERSION = "0.2.129"
CRATES = {
    "wasm-bindgen-cli-0.2.129": "5fd044ed178958a277eeaf21f3d384287b0f4428d38c019141df01da9c03168c",
    "tiny_http-0.12.0": "389915df6413a2e74fb181895f933386023c71110878cd0825588928e64cdc82",
}
BINS = ("wasm-bindgen", "wasm-bindgen-test-runner")
ROOT = Path(__file__).resolve().parent.parent
# The stock tiny_http queue condition. It is asserted, never rewritten: this build
# must stay readable as evidence about the current (unpatched) code.
STOCK_CONDITION = "if self.sharing.waiting_tasks.load(Ordering::Acquire) == 0 {"
PATCHED_CONDITION = "if self.sharing.waiting_tasks.load(Ordering::Acquire) <= queue.len() {"
TINY_LOCK = '''name = "tiny_http"
version = "0.12.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "389915df6413a2e74fb181895f933386023c71110878cd0825588928e64cdc82"'''

CLI = "wasm-bindgen-cli-0.2.129/src"
RUNNER = CLI + "/wasm_bindgen_test_runner.rs"
HEADLESS = CLI + "/wasm_bindgen_test_runner/headless.rs"
SERVER = CLI + "/wasm_bindgen_test_runner/server.rs"
TASK_POOL = "tiny_http-0.12.0/src/util/task_pool.rs"

LAUNCH_LOG_MODULE = '''
pub(crate) mod launch_log {
    //! BoardStudio launch-wait experiment logging: logging only, no behavior
    //! change, inert unless BOARDSTUDIO_LAUNCH_LOG_DIR is set. Lines
    //! `wall_ms=.. mono_ms=.. pid=.. port=.. ev=.. k=v..` are appended to
    //! $BOARDSTUDIO_LAUNCH_LOG_DIR/runner-<pid>-<server-port>.log. Events logged
    //! before the server port is known are buffered and flushed by `set_port`.
    use std::io::Write;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::{Mutex, OnceLock};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    static PORT: AtomicU16 = AtomicU16::new(0);
    static START: OnceLock<Instant> = OnceLock::new();
    static BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static WRITE_LOCK: Mutex<()> = Mutex::new(());

    fn dir() -> Option<String> {
        std::env::var("BOARDSTUDIO_LAUNCH_LOG_DIR")
            .ok()
            .filter(|s| !s.is_empty())
    }

    pub(crate) fn sanitize(s: &str) -> String {
        s.chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect()
    }

    fn line(ev: &str, extra: &str, port: u16) -> String {
        let wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let mono = START.get_or_init(Instant::now).elapsed().as_millis();
        if extra.is_empty() {
            format!(
                "wall_ms={wall} mono_ms={mono} pid={} port={port} ev={ev}",
                std::process::id()
            )
        } else {
            format!(
                "wall_ms={wall} mono_ms={mono} pid={} port={port} ev={ev} {extra}",
                std::process::id()
            )
        }
    }

    fn append(dir: &str, port: u16, lines: &[String]) {
        let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _ = std::fs::create_dir_all(dir);
        let path = format!("{dir}/runner-{}-{port}.log", std::process::id());
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            // One write of whole lines (plus the write lock above) so lines
            // from concurrent threads -- tiny_http's included -- never split.
            let mut buf = String::new();
            for l in lines {
                buf.push_str(l);
                buf.push('\\n');
            }
            let _ = f.write_all(buf.as_bytes());
        }
    }

    pub(crate) fn log(ev: &str, extra: &str) {
        let Some(dir) = dir() else { return };
        let port = PORT.load(Ordering::Relaxed);
        if port == 0 {
            let l = line(ev, extra, 0);
            BUFFER
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(l);
        } else {
            append(&dir, port, &[line(ev, extra, port)]);
        }
    }

    pub(crate) fn set_port(port: u16) {
        let Some(dir) = dir() else { return };
        PORT.store(port, Ordering::Relaxed);
        // tiny_http is a separate crate in the same process; it reads this
        // variable to append its TaskPool lines to the same runner log file.
        std::env::set_var("BOARDSTUDIO_LAUNCH_LOG_PORT", port.to_string());
        let buffered: Vec<String> = BUFFER
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
            .map(|l| l.replacen(" port=0 ", &format!(" port={port} "), 1))
            .collect();
        if !buffered.is_empty() {
            append(&dir, port, &buffered);
        }
    }

    pub(crate) fn netlog_path() -> Option<String> {
        dir().map(|d| {
            format!(
                "{d}/netlog-{}-{}.json",
                std::process::id(),
                PORT.load(Ordering::Relaxed)
            )
        })
    }

    pub(crate) fn chromedriver_path() -> Option<String> {
        dir().map(|d| {
            format!(
                "{d}/chromedriver-{}-{}.log",
                std::process::id(),
                PORT.load(Ordering::Relaxed)
            )
        })
    }

    pub(crate) struct ReqGuard {
        url: String,
    }

    impl ReqGuard {
        pub(crate) fn new(url: &str) -> Self {
            Self {
                url: sanitize(url),
            }
        }
    }

    impl Drop for ReqGuard {
        fn drop(&mut self) {
            log("req_responded", &format!("url={}", self.url));
        }
    }
}
'''

TINY_HELPERS = '''
// BoardStudio launch-wait experiment logging: logging only, no behavior change,
// inert unless BOARDSTUDIO_LAUNCH_LOG_DIR is set. The runner process publishes its
// server port in BOARDSTUDIO_LAUNCH_LOG_PORT so these lines land in the same
// runner-<pid>-<port>.log file. mono_ms here starts at this crate's first log call
// in the process; wall_ms is the cross-crate time reference.
static LAUNCH_LOG_CONN: AtomicUsize = AtomicUsize::new(0);
static LAUNCH_LOG_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
static LAUNCH_LOG_START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn launch_log_dir() -> Option<String> {
    std::env::var("BOARDSTUDIO_LAUNCH_LOG_DIR")
        .ok()
        .filter(|s| !s.is_empty())
}

fn launch_log_port() -> String {
    std::env::var("BOARDSTUDIO_LAUNCH_LOG_PORT").unwrap_or_else(|_| "0".to_string())
}

fn launch_log_line(ev: &str, extra: &str) -> Option<String> {
    launch_log_dir()?;
    let wall = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let mono = LAUNCH_LOG_START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis();
    let port = launch_log_port();
    Some(if extra.is_empty() {
        format!(
            "wall_ms={wall} mono_ms={mono} pid={} port={port} ev={ev}",
            std::process::id()
        )
    } else {
        format!(
            "wall_ms={wall} mono_ms={mono} pid={} port={port} ev={ev} {extra}",
            std::process::id()
        )
    })
}

fn launch_log_write(line: Option<String>) {
    let (Some(dir), Some(line)) = (launch_log_dir(), line) else {
        return;
    };
    use std::io::Write;
    let _guard = LAUNCH_LOG_WRITE_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let _ = std::fs::create_dir_all(&dir);
    let path = format!(
        "{dir}/runner-{}-{}.log",
        std::process::id(),
        launch_log_port()
    );
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        // One write of the whole line so concurrent appends never split it.
        let _ = f.write_all(format!("{line}\\n").as_bytes());
    }
}

fn launch_log(ev: &str, extra: &str) {
    launch_log_write(launch_log_line(ev, extra));
}

fn launch_log_next_conn() -> usize {
    if launch_log_dir().is_none() {
        0
    } else {
        LAUNCH_LOG_CONN.fetch_add(1, Ordering::Relaxed) + 1
    }
}
'''

# Every edit is an exact-match, count-asserted (exactly once) replacement against
# the checksum-verified stock sources. All hunks are logging only.
EDITS = [
    # --- wasm_bindgen_test_runner.rs: launch_log module + rmain phase events ---
    (
        RUNNER,
        "mod deno;\nmod headless;\nmod node;\nmod server;\nmod shell;",
        "mod deno;\nmod headless;\nmod node;\nmod server;\nmod shell;\n" + LAUNCH_LOG_MODULE,
    ),
    (
        RUNNER,
        '    shell.status("Executing bindgen...");',
        '    launch_log::log("bindgen_start", "");\n    shell.status("Executing bindgen...");',
    ),
    (
        RUNNER,
        '        .generate(&tmpdir_path)\n        .context("executing `wasm-bindgen` over the Wasm file")?;',
        '        .generate(&tmpdir_path)\n        .context("executing `wasm-bindgen` over the Wasm file")?;\n    launch_log::log("bindgen_end", "");',
    ),
    (
        RUNNER,
        "            let addr = srv.server_addr();\n\n            // TODO: eventually",
        '            let addr = srv.server_addr();\n            launch_log::set_port(addr.port());\n            launch_log::log("server_start", &format!("addr={addr}"));\n\n            // TODO: eventually',
    ),
    # --- headless.rs: driver spawn, session, navigate, poll, net-log switch ---
    (
        HEADLESS,
        "use super::shell::Shell;",
        "use super::launch_log;\nuse super::shell::Shell;",
    ),
    (
        HEADLESS,
        '                let mut cmd = Command::new(path);\n                cmd.args(args).arg(format!("--port={}", driver_addr.port()));',
        '                let mut cmd = Command::new(path);\n'
        '                cmd.args(args).arg(format!("--port={}", driver_addr.port()));\n'
        "                if let Some(log_path) = launch_log::chromedriver_path() {\n"
        '                    cmd.arg("--verbose")\n'
        '                        .arg(format!("--log-path={log_path}"));\n'
        "                }\n"
        '                launch_log::log(\n'
        '                    "driver_spawn",\n'
        '                    &format!("driver_port={}", driver_addr.port()),\n'
        "                );",
    ),
    (
        HEADLESS,
        "    client.new_session(&driver, capabilities, timeouts.startup)?;",
        '    launch_log::log("session_sent", "");\n'
        "    client.new_session(&driver, capabilities, timeouts.startup)?;\n"
        '    launch_log::log("session_returned", "");',
    ),
    (
        HEADLESS,
        '''                cap.entry("goog:chromeOptions".to_string())
                    .or_insert_with(|| Json::Object(serde_json::Map::new()))
                    .as_object_mut()
                    .expect("goog:chromeOptions wasn't a JSON object")
                    .entry("args".to_string())
                    .or_insert_with(|| Json::Array(vec![]))
                    .as_array_mut()
                    .expect("args wasn't a JSON array")
                    .extend(vec![
                        Json::String("headless".to_string()),
                        // See https://stackoverflow.com/questions/50642308/
                        // for what this funky `disable-dev-shm-usage`
                        // option is
                        Json::String("disable-dev-shm-usage".to_string()),
                        Json::String("no-sandbox".to_string()),
                    ]);''',
        '''                cap.entry("goog:chromeOptions".to_string())
                    .or_insert_with(|| Json::Object(serde_json::Map::new()))
                    .as_object_mut()
                    .expect("goog:chromeOptions wasn't a JSON object")
                    .entry("args".to_string())
                    .or_insert_with(|| Json::Array(vec![]))
                    .as_array_mut()
                    .expect("args wasn't a JSON array")
                    .extend(vec![
                        Json::String("headless".to_string()),
                        // See https://stackoverflow.com/questions/50642308/
                        // for what this funky `disable-dev-shm-usage`
                        // option is
                        Json::String("disable-dev-shm-usage".to_string()),
                        Json::String("no-sandbox".to_string()),
                    ]);
                if let Some(net_log) = launch_log::netlog_path() {
                    cap.get_mut("goog:chromeOptions")
                        .and_then(|o| o.as_object_mut())
                        .and_then(|o| o.get_mut("args"))
                        .and_then(|a| a.as_array_mut())
                        .expect("args wasn't a JSON array")
                        .push(Json::String(format!("log-net-log={net_log}")));
                }''',
    ),
    (
        HEADLESS,
        '        let path = self.session_path("/url")?;\n        let _: Response = self.post(&path, &request, timeout)?;',
        '        let path = self.session_path("/url")?;\n'
        '        launch_log::log(\n'
        '            "navigate_sent",\n'
        '            &format!("url={}", launch_log::sanitize(url)),\n'
        "        );\n"
        "        let body = match self.post_raw(&path, &request, timeout) {\n"
        "            Ok(body) => body,\n"
        "            Err(e) => {\n"
        "                launch_log::log(\n"
        '                    "navigate_returned",\n'
        '                    &format!("error={}", launch_log::sanitize(&format!("{e:?}"))),\n'
        "                );\n"
        "                return Err(e);\n"
        "            }\n"
        "        };\n"
        "        launch_log::log(\n"
        '            "navigate_returned",\n'
        '            &format!("body={}", launch_log::sanitize(&body)),\n'
        "        );\n"
        "        let _: Response = serde_json::from_str(&body)?;",
    ),
    (
        HEADLESS,
        '    shell.status("Waiting for test to finish...");\n    let start = Instant::now();',
        '    shell.status("Waiting for test to finish...");\n    launch_log::log("first_poll", "");\n    let start = Instant::now();',
    ),
    (
        HEADLESS,
        '    if output_buf.contains("test result: ") {\n        // If the tests harness finished (either successfully or unsuccessfully)',
        '    if output_buf.contains("test result: ") {\n        launch_log::log("test_result_seen", "");\n        // If the tests harness finished (either successfully or unsuccessfully)',
    ),
    # --- server.rs: per-request received/responded in the Rouille handler ---
    (
        SERVER,
        "use super::{Cli, TestMode, Tests};",
        "use super::launch_log;\nuse super::{Cli, TestMode, Tests};",
    ),
    (
        SERVER,
        "    let srv = Server::new(addr, move |request| {\n        // The root path gets our canned `index.html`.",
        "    let srv = Server::new(addr, move |request| {\n"
        '        launch_log::log(\n'
        '            "req_received",\n'
        '            &format!("url={}", launch_log::sanitize(&request.url())),\n'
        "        );\n"
        "        let _launch_log_guard = launch_log::ReqGuard::new(&request.url());\n"
        "        // The root path gets our canned `index.html`.",
    ),
    # --- tiny_http task_pool.rs: spawn/queue logging, condition untouched ---
    (
        TASK_POOL,
        "use std::time::Duration;",
        "use std::time::Duration;\n" + TINY_HELPERS,
    ),
    (
        TASK_POOL,
        "    pub fn spawn(&self, code: Box<dyn FnMut() + Send>) {\n"
        "        let mut queue = self.sharing.todo.lock().unwrap();\n"
        "\n"
        "        if self.sharing.waiting_tasks.load(Ordering::Acquire) == 0 {\n"
        "            self.add_thread(Some(code));\n"
        "        } else {\n"
        "            queue.push_back(code);\n"
        "            self.sharing.condvar.notify_one();\n"
        "        }\n"
        "    }",
        "    pub fn spawn(&self, code: Box<dyn FnMut() + Send>) {\n"
        "        // Logging only: wrap the unchanged task so its start/end are visible.\n"
        "        let conn = launch_log_next_conn();\n"
        "        let mut code = code;\n"
        "        let code: Box<dyn FnMut() + Send> = Box::new(move || {\n"
        '            launch_log("task_start", &format!("conn={conn}"));\n'
        "            code();\n"
        '            launch_log("task_end", &format!("conn={conn}"));\n'
        "        });\n"
        "        let mut queue = self.sharing.todo.lock().unwrap();\n"
        "\n"
        "        // The spawn line is formatted (its timestamps taken) at the branch,\n"
        "        // but written only after the queue lock is released below, so log\n"
        "        // file I/O never extends the stock lock hold.\n"
        "        let spawn_line;\n"
        "        if self.sharing.waiting_tasks.load(Ordering::Acquire) == 0 {\n"
        "            spawn_line = launch_log_line(\n"
        '                "spawn",\n'
        "                &format!(\n"
        '                    "conn={conn} waiting={} queue={} branch=new_thread",\n'
        "                    self.sharing.waiting_tasks.load(Ordering::Acquire),\n"
        "                    queue.len()\n"
        "                ),\n"
        "            );\n"
        "            self.add_thread(Some(code));\n"
        "        } else {\n"
        "            spawn_line = launch_log_line(\n"
        '                "spawn",\n'
        "                &format!(\n"
        '                    "conn={conn} waiting={} queue={} branch=queued",\n'
        "                    self.sharing.waiting_tasks.load(Ordering::Acquire),\n"
        "                    queue.len()\n"
        "                ),\n"
        "            );\n"
        "            queue.push_back(code);\n"
        "            self.sharing.condvar.notify_one();\n"
        "        }\n"
        "        drop(queue);\n"
        "        launch_log_write(spawn_line);\n"
        "    }",
    ),
]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def apply_edits(work):
    for rel, old, new in EDITS:
        path = work / rel
        text = path.read_text()
        if text.count(old) != 1:
            raise ValueError(f"instrumentation edit did not match exactly once in {rel}: {old[:80]!r}")
        path.write_text(text.replace(old, new))
    task_pool = (work / TASK_POOL).read_text()
    if task_pool.count(STOCK_CONDITION) != 1:
        raise ValueError("stock tiny_http queue condition is not present exactly once after edits")
    if PATCHED_CONDITION in task_pool:
        raise ValueError("patched tiny_http queue condition must never appear in the instrumented build")


def build(output, crate_dir, offline, workspace_lock, sources_out):
    output = Path(output).absolute()
    if output.exists() or output.is_symlink():
        raise ValueError(f"output already exists: {output}")
    if any(c.isspace() or c == os.pathsep for c in str(output.resolve())):
        raise ValueError("output path must not contain whitespace or PATH separators")
    if sources_out is not None:
        sources_out = Path(sources_out).absolute()
        if sources_out.exists() or sources_out.is_symlink():
            raise ValueError(f"sources-out already exists: {sources_out}")
    versions = re.findall(r'name = "wasm-bindgen"\nversion = "([^"]+)"', Path(workspace_lock).read_text())
    if versions != [VERSION]:
        raise ValueError(f"workspace wasm-bindgen version must be exactly {VERSION}: {versions}")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".wasm-bindgen-build-", dir=output.parent) as tmp:
        work = Path(tmp)
        for name, expected in CRATES.items():
            candidates = ([Path(crate_dir) / (name + ".crate")] if crate_dir else
                          sorted((Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")) / "registry/cache").glob("*/" + name + ".crate")))
            archive = next((p for p in candidates if p.is_file()), None)
            if archive is None:
                if offline:
                    raise ValueError(f"offline crate archive not found: {name}")
                archive = work / (name + ".crate")
                urllib.request.urlretrieve(f"https://static.crates.io/crates/{name.rsplit('-', 1)[0]}/{name}.crate", archive)
            if digest(archive) != expected:
                raise ValueError(f"crate checksum mismatch: {archive}")
            with tarfile.open(archive, "r:gz") as tar:
                tar.extractall(work, filter="data")
        source = work / "wasm-bindgen-cli-0.2.129"
        tiny = work / "tiny_http-0.12.0"
        task_pool_text = (work / TASK_POOL).read_text()
        if task_pool_text.count(STOCK_CONDITION) != 1:
            raise ValueError("stock tiny_http queue condition not found exactly once in stock source")
        apply_edits(work)
        lock = source / "Cargo.lock"
        text = lock.read_text()
        if text.count(TINY_LOCK) != 1:
            raise ValueError("upstream lockfile tiny_http entry did not match")
        # A path-patched package has no registry source/checksum; all other pins stay intact.
        lock.write_text(text.replace(TINY_LOCK, 'name = "tiny_http"\nversion = "0.12.0"'))
        if sources_out is not None:
            sources_out.mkdir(parents=True)
            shutil.copytree(source, sources_out / source.name)
            shutil.copytree(tiny, sources_out / tiny.name)
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(work / "target")
        env["RUSTUP_TOOLCHAIN"] = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
        command = ["cargo", "--config", str(ROOT / ".cargo/config.toml"), "build", "--release", "--locked"]
        if offline:
            command.append("--offline")
        command += ["--config", f"patch.crates-io.tiny_http.path={json.dumps(str(tiny))}"]
        for name in BINS:
            command += ["--bin", name]
        subprocess.run(command, cwd=source, env=env, check=True)
        pair = work / "pair"
        pair.mkdir()
        for name in BINS:
            binary = work / "target/release" / name
            result = subprocess.run([str(binary), "--version"], capture_output=True, text=True, check=True)
            if result.stdout.strip() != f"{name} {VERSION}":
                raise ValueError(f"unexpected version from {binary}: {result.stdout.strip()!r}")
            shutil.copy2(binary, pair / name)
        pair.rename(output)
    print(f"built instrumented pair: {output}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True, help="new directory for the two binaries")
    parser.add_argument("--crate-dir", type=Path, help="directory containing both .crate archives")
    parser.add_argument("--offline", action="store_true", help="do not download crates or resolve dependencies online")
    parser.add_argument("--sources-out", type=Path, help="new directory to also copy the instrumented crate sources into (for diffing against stock)")
    args = parser.parse_args()
    try:
        build(args.output, args.crate_dir, args.offline, Path(__file__).resolve().parent.parent / "Cargo.lock", args.sources_out)
    except (OSError, ValueError, subprocess.SubprocessError, tarfile.TarError) as error:
        print(f"build-instrumented-wasm-bindgen: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
