#!/usr/bin/env python3
"""020 review regressions on the packaged page and actual public Inspector composition."""
import argparse
import importlib
import json
import os
from pathlib import Path
import subprocess
from uuid import uuid4


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url")
    parser.add_argument("--only", choices=("assembly", "controller", "attachment"), required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--width", type=int, default=390)
    parser.add_argument("--coarse", action="store_true")
    parser.add_argument("--theme", choices=("original", "shadcn"), default="original")
    parser.add_argument("--appearance", choices=("light", "dark"), default="light")
    parser.add_argument("--build-inspector", action="store_true")
    args = parser.parse_args()
    if args.build_inspector:
        root = Path(__file__).resolve().parents[1]
        out = root / "web/target/site/site-subpath/boardstudio/review-inspector"
        subprocess.run(["cargo", "build", "--locked", "-p", "boardstudio-web-parts", "--example", "inspector_020",
                        "--target", "wasm32-unknown-unknown"], cwd=root, check=True,
                       env={**os.environ, "CARGO_PROFILE_DEV_DEBUG": "false"})
        out.mkdir(parents=True, exist_ok=True)
        bindgen = importlib.import_module("run-wasm-tests").wasm_bindgen_dir(root) / "wasm-bindgen"
        subprocess.run([str(bindgen), "--target", "web", "--out-dir", str(out),
                        str(root / "target/wasm32-unknown-unknown/debug/examples/inspector_020.wasm")], check=True)
        subprocess.run([str(Path.home() / ".local/share/.dx/tools/tailwindcss-v4.1.5/tailwindcss"),
                        "-i", str(root / "web/tailwind.css"), "-o", str(out / "tailwind.css")], check=True)
        if not (out / "assets").exists():
            (out / "assets").symlink_to("../assets", target_is_directory=True)
        (out / "index.html").write_text('<!doctype html><html lang="en"><meta charset="utf-8">'
            '<meta name="viewport" content="width=device-width,initial-scale=1"><title>Actual Parts Inspector</title>'
            '<link rel="stylesheet" href="../assets/m1.css"><link rel="stylesheet" href="tailwind.css">'
            '<body><div id="main"></div><script type="module">import init from "./inspector_020.js";init();</script>')
    args.evidence.mkdir(parents=True, exist_ok=True)
    driver = importlib.import_module("test-dock-primitives")
    driver.SESSION = "parts-review-020-" + uuid4().hex
    browser, evaluate, check = driver.browser, driver.evaluate, driver.check

    def action(role, name, verb="click", *values):
        return browser("find", "role", role, verb, *values, "--name", name, "--exact")

    def focus(role, name):
        browser("focus", driver.by_role_name(role, name))

    def capture(name, metrics):
        print(name, json.dumps(metrics), flush=True)
        (args.evidence / f"{name}.json").write_text(json.dumps(metrics, indent=2) + "\n")
        browser("screenshot", str((args.evidence / f"{name}.png").resolve()))

    def box(expression):
        return evaluate(f"(()=>{{const e={expression},r=e.getBoundingClientRect();return {{x:r.x,y:r.y,width:r.width,height:r.height,disabled:e.disabled}}}})()")

    def touch_targets(root):
        metrics = evaluate(f"""[...document.querySelectorAll('{root} [data-slot=button],{root} [data-slot=collapsible-trigger],{root} [data-slot=input-group-control],{root} [data-slot=select-trigger]')].filter(e=>e.checkVisibility()).map(e=>{{const r=e.getBoundingClientRect();return {{name:e.getAttribute('aria-label')||e.textContent.trim(),slot:e.dataset.slot,width:r.width,height:r.height}}}})""")
        capture("related-touch-targets", metrics)
        if args.width <= 960 or args.coarse:
            assert all(item["width"] >= 44 and item["height"] >= 44 for item in metrics), metrics

    try:
        if args.coarse:
            browser("set", "device", "iPhone 12")
        browser("set", "viewport", args.width if args.only == "attachment" else 1440, 960 if args.width > 960 else 844)
        browser("open", args.url)
        assert evaluate("matchMedia('(pointer: coarse)').matches") == args.coarse
        if args.only == "attachment":
            browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Assign host connection\"]')")
            evaluate(f"document.documentElement.dataset.theme={json.dumps(args.theme)};document.documentElement.dataset.appearance={json.dumps(args.appearance)}")
            focus("checkbox", "Assign host connection")
            evaluate("window.connectionControl=document.activeElement;window.connectionLabel=[...document.querySelectorAll('label[for]')].find(e=>e.htmlFor===connectionControl.id)")
            evaluate("document.fonts.ready.then(()=>true)")
            metrics = evaluate("""(()=>{const r=connectionLabel.getBoundingClientRect(),c=connectionControl.getBoundingClientRect();return {label:[r.x,r.y,r.width,r.height],checkbox:[c.width,c.height],checked:connectionControl.getAttribute('aria-checked'),disabled:connectionControl.disabled}})()""")
            capture("attachment-target", metrics)
            assert metrics["label"][3] == (44 if args.width <= 960 or args.coarse else 16), metrics
            assert metrics["checkbox"] == [16, 16] and not metrics["disabled"] and metrics["checked"] == "true", metrics
            touch_targets(".m1-module-attach")
            browser("press", "Enter")
            check("connectionControl.getAttribute('aria-checked')==='true'")
            browser("press", "Space")
            browser("wait", "--fn", "connectionControl.getAttribute('aria-checked')==='false'")
            action("button", "Refresh parent")
            focus("checkbox", "Assign host connection")
            check("document.activeElement===connectionControl && connectionControl.getAttribute('aria-checked')==='false'")
            # Exercise the enlarged label outside the 16px checkbox with trusted native activation.
            evaluate("document.fonts.ready.then(()=>true)")
            point = evaluate("(()=>{const r=connectionLabel.getBoundingClientRect();return {x:r.right-4,y:r.y+r.height/2}})()")
            browser("mouse", "move", round(point["x"]), round(point["y"]))
            check(f"connectionLabel.contains(document.elementFromPoint({round(point['x'])},{round(point['y'])}))")
            evaluate("window.connectionClicks=[];connectionControl.addEventListener('click',e=>connectionClicks.push(e.isTrusted))")
            browser("mouse", "down")
            browser("mouse", "up")
            browser("wait", "--fn", "connectionControl.getAttribute('aria-checked')==='true'")
            check("connectionClicks.length===1 && connectionClicks[0]")
            focus("button", "Module host connector")
            browser("press", "Enter")
            browser("press", "Home")
            browser("press", "Enter")
            evaluate("window.connectorControl=document.activeElement;window.connectorValue=connectorControl.textContent")
            action("button", "Refresh parent")
            check("connectorControl.isConnected && connectorControl.textContent===connectorValue")
            action("button", "Choose module without port")
            browser("wait", "--fn", "document.querySelector('[aria-label=\"Assign host connection\"]')?.disabled")
            evaluate("window.disabledConnection=document.querySelector('[aria-label=\"Assign host connection\"]');window.disabledBefore=disabledConnection.getAttribute('aria-checked')")
            disabled = box("disabledConnection")
            disabled_label = box("disabledConnection.closest('label')")
            assert disabled_label["height"] >= (44 if args.width <= 960 or args.coarse else 16), disabled_label
            check("disabledConnection.closest('label').control===disabledConnection")
            point = evaluate("(()=>{const r=disabledConnection.closest('label').getBoundingClientRect();return {x:r.right-4,y:r.y+r.height/2}})()")
            browser("mouse", "move", round(point["x"]), round(point["y"]))
            browser("mouse", "down")
            browser("mouse", "up")
            check("disabledConnection.getAttribute('aria-checked')===disabledBefore")
            print("Actual retained local connection draft, connector value and disabled module branch:", disabled, flush=True)
        else:
            browser("wait", "--fn", "!!document.querySelector('[aria-label=Project][aria-haspopup=menu]')")
            action("button", "Project")
            action("menuitem", "New project")
            browser("wait", "--fn", "document.querySelector('[aria-label=\"Keyboard setup\"]')?.checkVisibility()")
            action("button", "Settings")
            action("menuitemradio", "Original" if args.theme == "original" else "shadcn")
            action("button", "Settings")
            action("menuitemradio", args.appearance.title())
            browser("wait", "--fn", f"document.documentElement.dataset.theme==={json.dumps(args.theme)} && document.documentElement.dataset.appearance==={json.dumps(args.appearance)}")
            if args.only == "controller":
                action("button", "3 Controller & wiring")
                browser("set", "viewport", args.width, 960 if args.width > 960 else 844)
                browser("wait", "--fn", "!!document.querySelector('.m1-controller-card')")
                browser("wait", "--fn", "document.querySelectorAll('.m1-controller-footprint svg').length>=2", "--timeout", 15000)
                evaluate("document.fonts.ready.then(()=>true)")
                metrics = evaluate("""[...document.querySelectorAll('.m1-controller-card')].filter(e=>e.getAttribute('data-match')==='true').map(e=>{const rect=n=>{const r=n.getBoundingClientRect();return [r.left,r.top,r.right,r.bottom,r.width,r.height]};return {name:e.textContent.trim(),pressed:e.getAttribute('aria-pressed'),card:rect(e),thumbnail:rect(e.querySelector('.m1-controller-footprint')),label:rect(e.querySelector('strong'))}})""")
                capture("controller-cards", metrics)
                navigation = evaluate("[...document.querySelectorAll('[aria-label=\"Previous controller\"],[aria-label=\"Next controller\"]')].map(e=>{const r=e.getBoundingClientRect();return {name:e.getAttribute('aria-label'),width:r.width,height:r.height}})")
                print("Navigation inventory:", navigation, flush=True)
                if args.width <= 960 or args.coarse:
                    assert all(item["width"] >= 44 and item["height"] >= 44 for item in navigation), navigation
                assert len(metrics) >= 2, metrics
                for item in metrics:
                    for child in (item["thumbnail"], item["label"]):
                        assert child[0] >= item["card"][0] and child[1] >= item["card"][1] and child[2] <= item["card"][2] and child[3] <= item["card"][3], item
                    assert item["thumbnail"][5] == 132 and item["label"][5] > 0, item
                target = next(item for item in metrics if item["pressed"] == "false")
                action("img", target["name"] + " footprint")
                browser("wait", "--fn", f"[...document.querySelectorAll('.m1-controller-card')].find(e=>e.textContent.trim()==={json.dumps(target['name'])}).getAttribute('aria-pressed')==='true'")
                check("document.querySelectorAll('.m1-controller-card[aria-pressed=true]').length===1")
                for name in ("Previous controller", "Next controller"):
                    focus("button", name)
                    metrics = box("document.activeElement")
                    print("Navigation target:", name, metrics, flush=True)
                    assert metrics["width"] >= (44 if args.width <= 960 or args.coarse else 36), metrics
                    browser("press", "Space")
                action("searchbox", "Search controllers", "fill", target["name"])
                browser("wait", "--fn", "document.querySelectorAll('.m1-controller-card[data-match=true]').length===1")
                check("document.querySelector('[aria-label=\"Previous controller\"]').disabled && document.querySelector('[aria-label=\"Next controller\"]').disabled")
            else:
                action("button", "4 Case (optional)")
                action("button", "Hide guide")
                action("button", "Open Parts workspace")
                browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Search footprints\"]')")
                action("button", "MX Solder")
                browser("set", "viewport", args.width, 960 if args.width > 960 else 844)
                if args.width < 980:
                    action("button", "Workbench menu")
                    action("button", "Objects")
                    action("tab", "Objects")
                focus("button", "New assembly")
                metrics = box("document.activeElement")
                capture("assembly-target", metrics)
                assert metrics["width"] == metrics["height"] == (44 if args.width <= 960 or args.coarse else 36), metrics
                assert not metrics["disabled"], metrics
                touch_targets(".m1-parts-library")
                modes = evaluate("[...document.querySelectorAll('[aria-label=\"2D footprint\"],[aria-label=\"3D model\"]')].map(e=>{const r=e.getBoundingClientRect();return {width:r.width,height:r.height}})")
                assert len(modes) == 2, modes
                if args.width <= 960 or args.coarse:
                    assert all(item["width"] >= 44 and item["height"] >= 44 for item in modes), modes
                focus("button", "Switch orientation")
                browser("press", "Enter")
                options = evaluate("[...document.querySelectorAll('[role=option]')].filter(e=>e.checkVisibility()).map(e=>{const r=e.getBoundingClientRect();return {name:e.textContent,width:r.width,height:r.height}})")
                assert len(options) == 2, options
                if args.width <= 960 or args.coarse:
                    assert all(item["width"] >= 44 and item["height"] >= 44 for item in options), options
                browser("press", "End")
                browser("press", "Enter")
                check("document.activeElement.getAttribute('aria-label')==='Switch orientation' && document.activeElement.textContent.includes('North-facing')")
                focus("button", "New assembly")
                evaluate("window.newAssembly=document.activeElement;window.assemblySection=document.querySelector('[aria-label=\"Key assemblies\"]')?.getAttribute('aria-expanded')")
                action("button", "New assembly")
                browser("wait", "--fn", "[...document.querySelectorAll('button')].some(e=>e.textContent.trim()==='Close editor')")
                check("newAssembly.isConnected && document.querySelector('[aria-label=\"Key assemblies\"]')?.getAttribute('aria-expanded')===assemblySection")
                if args.width < 980:
                    action("tab", "Inspector")
                action("button", "Close editor")
                if args.width < 980:
                    action("tab", "Objects")
                focus("button", "New assembly")
                browser("press", "Enter")
                if args.width < 980:
                    action("tab", "Inspector")
                action("button", "Close editor")
        print("PASS:", args.only, "width", args.width, "coarse", args.coarse, flush=True)
    except Exception:
        browser("screenshot", str((args.evidence / "failure.png").resolve()))
        raise
    finally:
        browser("close")


if __name__ == "__main__":
    main()
