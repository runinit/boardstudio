#!/usr/bin/env python3
"""Trusted-key browser tests of the same mounted fixture as the wasm tests.

Build the example, then pass its .wasm and the pinned wasm-bindgen:
  cargo build --locked -p boardstudio-web-ui-shared --target wasm32-unknown-unknown --example ticket_008_fixture
  python3 web/crates/ui-shared/ticket-008-keyboard.py target/wasm32-unknown-unknown/debug/examples/ticket_008_fixture.wasm /path/to/wasm-bindgen
Generated browser artifacts go to .evidence/008/browser. Uses the repository's agent-browser workflow.
"""
import functools
import http.server
import json
from pathlib import Path
import subprocess
import sys
import threading

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / '.evidence/008/browser'
SESSION = 'ticket-008-' + ROOT.name


def browser(*args):
    result = subprocess.run(['agent-browser', '--session', SESSION, '--json', *args],
                            text=True, capture_output=True, timeout=35)
    assert result.returncode == 0, result.stdout + result.stderr
    response = json.loads(result.stdout)
    assert response['success'], response
    return response.get('data', {})


def evaluate(js):
    return browser('eval', js).get('result')


def expect(js):
    assert evaluate(js), js


def wait(js):
    browser('wait', '--fn', js)


def focus(role, name):
    evaluate(f'window.control({json.dumps(role)}, {json.dumps(name)}).focus()')


def press(key):
    browser('press', key)


def value(name, expected):
    wait(f'control("slider", {json.dumps(name)}).getAttribute("aria-valuenow") === "{expected}"')


def focused(name):
    wait(f'document.activeElement === control("button", {json.dumps(name)})')


def toolbar():
    focus('button', 'Before toolbar'); press('Tab'); focused('Undo')
    expect('control("toolbar", "History").querySelectorAll("[tabindex=\\"0\\"]").length === 1')
    press('ArrowRight'); focused('Redo')
    press('End'); focused('Last action')
    press('Home'); focused('Undo')
    press('Enter'); press('Space')
    expect('control("status", "Action count").textContent === "2"')
    press('ArrowRight'); focused('Redo'); press('Tab'); focused('After toolbar')
    press('Shift+Tab'); focused('Redo')
    focus('button', 'Top tool'); press('ArrowDown'); focused('Bottom tool')
    press('Home'); focused('Top tool'); press('End'); focused('Bottom tool')
    expect('control("button", "Disabled action").disabled')


def toggle():
    expect('!!control("group", "Alignment")')
    focus('button', 'Left'); press('Space')
    wait('control("button", "Left").getAttribute("aria-pressed") === "true"')
    press('ArrowRight'); focused('Right')
    expect('control("button", "Right").getAttribute("aria-pressed") === "false"')
    press('Enter')
    wait('control("button", "Right").getAttribute("aria-pressed") === "true"')
    expect('control("button", "Left").getAttribute("aria-pressed") === "false"')
    press('ArrowRight'); focused('Left'); press('End'); focused('Right'); press('Home'); focused('Left')
    focus('button', 'Bold'); press('Space'); press('ArrowDown'); focused('Italic'); press('Enter')
    wait('control("button", "Italic").getAttribute("aria-pressed") === "true"')
    expect('control("button", "Bold").getAttribute("aria-pressed") === "true"')
    expect('control("status", "Toggle changes").textContent === "4"')


def slider():
    for name in ['Zoom', 'Controlled zoom']:
        focus('slider', name)
        expect(f'control("slider", "{name}").getAttribute("aria-valuemin") === "10"')
        expect(f'control("slider", "{name}").getAttribute("aria-valuemax") === "90"')
        expect(f'control("slider", "{name}").getAttribute("aria-orientation") === "horizontal"')
        for key, expected in [('ArrowRight', 42), ('ArrowLeft', 40), ('ArrowUp', 42), ('ArrowDown', 40),
                              ('Home', 10), ('End', 90), ('PageDown', 70), ('PageUp', 90)]:
            press(key); value(name, expected)
    expect('control("status", "Slider changes").textContent === "[42.0, 40.0, 42.0, 40.0, 10.0, 90.0, 70.0, 90.0, 42.0, 40.0, 42.0, 40.0, 10.0, 90.0, 70.0, 90.0]"')
    expect('control("slider", "Disabled zoom").disabled')
    focus('slider', 'Controlled zoom'); press('Tab'); focused('Notify')
    expect('control("slider", "Disabled zoom").getAttribute("aria-valuenow") === "40"')


def toast():
    focus('button', 'Notify'); press('Enter')
    wait('!!control("button", "close")')
    expect('!document.querySelector("[role=alertdialog]")')
    expect('control("status", "Saved").getAttribute("aria-atomic") === "true"')
    expect('control("status", "Saved").textContent.includes("Project stored")')
    focused('Notify')
    press('Tab'); focused('After notifications'); press('Tab'); focused('close')
    press('Escape')
    expect('!!control("status", "Saved")')  # Passive notification: Escape is not captured.
    press('Enter'); wait('!control("status", "Saved")')
    expect('control("status", "Notification actions").textContent === "1"')
    focus('button', 'Notify'); press('Space'); wait('!!control("button", "close")')
    focus('button', 'close'); press('Space'); wait('!control("status", "Saved")')
    expect('control("status", "Notification actions").textContent === "2"')


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    subprocess.run([sys.argv[2], '--target', 'web', '--out-dir', str(OUT), '--out-name', 'fixture', sys.argv[1]], check=True)
    (OUT / 'index.html').write_text('''<!doctype html><html lang="en"><title>Ticket 008 mounted keyboard fixture</title>
<script type="module">
import init, {mount_ticket_008_fixture, ticket_008_control} from './fixture.js';
try {
    await init();
    window.control = ticket_008_control;
    mount_ticket_008_fixture();
} catch (error) { window.fixtureError = String(error); }
</script></html>''')
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=OUT)
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    url = f'http://127.0.0.1:{server.server_port}/'
    failures = []
    try:
        for test in [toolbar, toggle, slider, toast]:
            try:
                browser('open', url)
                wait('window.fixtureError || (typeof window.control === "function" && !!control("slider", "Zoom"))')
                assert not evaluate('window.fixtureError'), evaluate('window.fixtureError')
                test()
                print(f'PASS {test.__name__}', flush=True)
            except (AssertionError, subprocess.SubprocessError) as error:
                failures.append(test.__name__)
                print(f'FAIL {test.__name__}: {error}', flush=True)
        assert not failures, failures
    finally:
        browser('close')
        server.shutdown()


if __name__ == '__main__':
    main()
