#!/usr/bin/env python3
"""Ticket 014: selected PCB component Position styles and native edit semantics."""
import argparse
import json
from pathlib import Path
import subprocess
from uuid import uuid4


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url")
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    session = "position-014-" + uuid4().hex

    def browser(*command):
        result = subprocess.run(["agent-browser", "--session", session, "--json", *command],
                                text=True, capture_output=True, timeout=90)
        assert result.returncode == 0, (command, result.stdout, result.stderr)
        reply = json.loads(result.stdout)
        assert reply["success"], reply
        return reply["data"]

    def action(role, name, verb="click", value=None):
        if verb == "focus":
            refs = browser("snapshot", "-i")["refs"]
            found = [ref for ref, item in refs.items()
                     if item.get("role") == role and item.get("name") == name]
            assert len(found) == 1, (role, name, found)
            return browser("focus", "@" + found[0])
        command = ["find", "role", role, verb]
        if value is not None:
            command.append(value)
        return browser(*command, "--name", name, "--exact")

    def evaluate(js):
        return browser("eval", js)["result"]

    def dismiss_notices():
        for ref, item in browser("snapshot", "-i")["refs"].items():
            if item.get("role") == "button" and item.get("name") == "close":
                browser("click", "@" + ref)

    observations = []
    try:
        browser("set", "viewport", "1440", "960")
        browser("open", args.url)
        browser("wait", "--text", "Your keyboards")
        action("button", "Project")
        action("menuitem", "New project")
        for name in ("Continue to layout", "Hide guide", "Cancel"):
            browser("wait", "--text", name)
            action("button", name)
        action("button", "Add key matrix")
        action("button", "Create matrix")
        browser("wait", "--text", "Matrix 1")
        action("button", "Open PCB workspace")
        action("button", "SW1, switch mx, X 0.00 Y 0.00")
        action("button", "Selection")
        for theme in ("original", "shadcn"):
            action("button", "Settings")
            action("menuitemradio", "Original" if theme == "original" else "shadcn")
            for appearance in ("light", "dark"):
                action("button", "Settings")
                action("menuitemradio", appearance.title())
                browser("wait", "--fn", f"document.documentElement.dataset.theme === '{theme}' && document.documentElement.dataset.appearance === '{appearance}'")
                fields = []
                for name in ("X (mm)", "Y (mm)"):
                    action("spinbutton", name, "focus")
                    fields.append(evaluate("""(async () => {
                        const input = document.activeElement;
                        if (input.dataset.slot !== 'input-group-control') throw Error('actual copied Position input required');
                        const label = input.labels[0], group = input.parentElement;
                        const probe = document.createElement('div');
                        probe.style.cssText = 'position:fixed;visibility:hidden;color:var(--foreground);width:320px';
                        const referenceLabel = label.cloneNode(true);
                        referenceLabel.removeAttribute('for');
                        const referenceGroup = group.cloneNode(true);
                        const referenceInput = referenceGroup.firstElementChild;
                        referenceInput.removeAttribute('id');
                        probe.append(referenceLabel, referenceGroup);
                        document.body.append(probe);
                        try {
                            await Promise.allSettled([input, label, group, referenceInput, referenceLabel, referenceGroup]
                                .flatMap(el => el.getAnimations()).map(animation => animation.finished));
                            const read = (el, names) => Object.fromEntries(names.map(name => [name, getComputedStyle(el)[name]]));
                            const inputProperties = ['backgroundColor', 'borderTopLeftRadius', 'color', 'marginTop', 'marginBottom', 'height'];
                            const labelProperties = ['display', 'color', 'fontSize', 'fontWeight', 'marginTop', 'marginBottom'];
                            return {name:label.textContent, type:input.type, step:input.step, value:input.value,
                                parentColor:getComputedStyle(input.closest('[data-slot=field]')).color,
                                actualInput:read(input,inputProperties), expectedInput:read(referenceInput,inputProperties),
                                actualLabel:read(label,labelProperties), expectedLabel:read(referenceLabel,labelProperties),
                                actualGroup:read(group,['backgroundColor']), expectedGroup:read(referenceGroup,['backgroundColor'])};
                        } finally { probe.remove(); }
                    })()"""))
                # A real unmigrated caller of the shared inspector rule must keep its look.
                action("button", "Routing & nets")
                action("combobox", "Wiring mode", "focus")
                legacy = evaluate("""(() => {
                    const input = document.activeElement;
                    const probe = document.createElement('div');
                    probe.style.cssText = 'background:var(--wb-field);border-radius:var(--wb-radius-control);min-height:var(--wb-control-height)';
                    document.body.append(probe);
                    try {
                        const read = el => { const css=getComputedStyle(el); return {backgroundColor:css.backgroundColor,borderTopLeftRadius:css.borderTopLeftRadius,minHeight:css.minHeight}; };
                        return {slot:input.dataset.slot || null, actual:read(input),expected:read(probe)};
                    } finally { probe.remove(); }
                })()""")
                action("button", "Selection")
                dismiss_notices()
                browser("screenshot", str((args.evidence / f"position-{theme}-{appearance}.png").resolve()))
                observations.append({"theme": theme, "appearance": appearance, "fields": fields, "unmigratedWiringMode": legacy})
        failures = []
        for state in observations:
            for field in state["fields"]:
                assert field["type"] == "number" and field["step"] == "any", field
                for target in ("Input", "Label", "Group"):
                    if field["actual" + target] != field["expected" + target]:
                        failures.append({"theme":state["theme"], "appearance":state["appearance"],
                                         "field":field["name"], "target":target,
                                         "actual":field["actual"+target], "expected":field["expected"+target]})
            assert state["unmigratedWiringMode"]["slot"] is None, state
            assert state["unmigratedWiringMode"]["actual"] == state["unmigratedWiringMode"]["expected"], state
        (args.evidence / "computed.json").write_text(json.dumps({"observations": observations, "failures": failures}, indent=2) + "\n")
        assert not failures, json.dumps(failures, indent=2)
        # The same production selected component keeps precise native values and one Undo step.
        action("spinbutton", "X (mm)", "fill", "0.1234567890123456")
        assert evaluate("({value:document.activeElement.value,valid:document.activeElement.validity.valid})") == {"value": "0.1234567890123456", "valid": True}
        browser("press", "Escape")
        browser("wait", "--fn", "document.activeElement?.value === '0'")
        action("spinbutton", "X (mm)", "fill", "0.1234567890123456")
        browser("press", "Enter")
        browser("wait", "--fn", "!!document.querySelector('[aria-label=\"Saved locally\"]') && document.activeElement?.value === '0.1234567890123456'")
        action("button", "Saved locally", "focus")
        browser("press", "Control+z")
        action("spinbutton", "X (mm)", "focus")
        browser("wait", "--fn", "document.activeElement?.value === '0'")
        (args.evidence / "result.json").write_text(json.dumps({"result": "passed", "combinations": 4,
            "coverage": "Actual selected SW1 in PCB Selection Position; copied-style equality, legacy Wiring mode, native precision, Escape/Enter and Undo"}, indent=2) + "\n")
        print("PASS: actual Position fields in four Theme/Appearance combinations; legacy caller unchanged; native precision/Escape/Enter/Undo")
    except BaseException:
        (args.evidence / "partial-computed.json").write_text(json.dumps(observations, indent=2) + "\n")
        (args.evidence / "failure-snapshot.json").write_text(json.dumps(browser("snapshot", "-i"), indent=2) + "\n")
        browser("screenshot", str((args.evidence / "failure.png").resolve()))
        raise
    finally:
        browser("close")


if __name__ == "__main__":
    main()
