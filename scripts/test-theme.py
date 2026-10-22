#!/usr/bin/env python3
"""Check the packaged Theme contract through agent-browser (serve the lane build first)."""
import argparse
import atexit
from pathlib import Path
import subprocess
from uuid import uuid4


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url")
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    session = "theme-002-" + uuid4().hex

    def browser(*command):
        result = subprocess.run(
            ["agent-browser", "--session", session, *command],
            text=True, capture_output=True, timeout=90,
        )
        assert result.returncode == 0, f"{command}: {result.stdout}{result.stderr}"
        return result.stdout

    browser("open", args.url)
    atexit.register(browser, "close")
    browser("wait", '[aria-label="Create new keyboard"]')
    browser("find", "role", "button", "click", "--name", "Create new keyboard", "--exact")
    for name in ("Continue to layout", "Hide guide", "Cancel"):
        browser("wait", "--text", name)
        browser("find", "role", "button", "click", "--name", name, "--exact")
    browser("wait", "--text", "Browse parts")
    observations = []
    for theme in ("original", "shadcn"):
        browser("find", "role", "button", "click", "--name", "Settings", "--exact")
        browser("find", "role", "menuitemradio", "click", "--name", "Original" if theme == "original" else "shadcn", "--exact")
        for appearance in ("light", "dark"):
            browser("find", "role", "button", "click", "--name", "Settings", "--exact")
            browser("find", "role", "menuitemradio", "click", "--name", appearance.title(), "--exact")
            browser("wait", "--fn", f"document.documentElement.dataset.theme === '{theme}' && document.documentElement.dataset.appearance === '{appearance}'")
            result = browser("eval", """(async () => {
                const root = document.documentElement;
                const css = getComputedStyle(root);
                const value = name => css.getPropertyValue('--' + name).trim();
                const tokens = `background foreground card card-foreground popover popover-foreground primary primary-foreground primary-dark primary-light secondary secondary-foreground muted muted-foreground accent accent-foreground border input ring destructive destructive-foreground destructive-light destructive-dark success success-foreground success-light success-dark warning warning-foreground warning-light warning-dark info info-foreground info-light info-dark selection selection-foreground sidenav sidenav-foreground sidenav-primary sidenav-primary-foreground sidenav-accent sidenav-accent-foreground sidenav-border sidenav-ring slider-track scrollbar-thumb scrollbar-track surface surface-foreground canvas selected selected-foreground field border-strong toolbar toolbar-border geometry-outline geometry-fill geometry-fill-muted geometry-key geometry-part geometry-label geometry-body geometry-board geometry-graphic geometry-zone geometry-text grid-small grid-large keycap-face keycap-ink keycap-rule icon-stroke radius theme-font-sans theme-font-mono theme-shadow-xs theme-shadow-sm theme-shadow-md theme-shadow-lg`.split(' ');
                for (const token of tokens) if (!value(token)) throw Error('Missing token: ' + token);
                for (const [token, expected] of [['control-height','40px'], ['control-compact','36px'], ['control-touch','44px']]) {
                    if (value(token) !== expected) throw Error(token + ': ' + value(token));
                }
                for (const token of tokens.filter(t => t.startsWith('geometry-') || t.startsWith('grid-'))) {
                    if (value('wb-' + token) !== value(token)) throw Error('Legacy alias: ' + token);
                }
                if (value('wb-canvas') !== value('canvas')) throw Error('Legacy canvas alias');
                const canvas = document.querySelector('[role="group"][aria-label^="Keyboard layout"]');
                if (!canvas) throw Error('Mounted layout canvas missing');
                const probe = document.createElement('span');
                probe.style.backgroundColor = 'var(--canvas)';
                document.body.append(probe);
                try {
                    if (getComputedStyle(canvas).backgroundColor !== getComputedStyle(probe).backgroundColor) throw Error('Mounted canvas background');
                } finally { probe.remove(); }
                if (value('background') === value('card')) throw Error('Desk equals card');
                if (root.dataset.theme === 'original') {
                    const dark = root.dataset.appearance === 'dark';
                    for (const [token, expected] of Object.entries(dark ? {
                        background:'#181B21', card:'#242831', primary:'#9AADFF', field:'#20242D', selected:'#303B56', 'geometry-key':'#9AADFF', 'grid-small':'#252A32'
                    } : {
                        background:'#E9EEF2', card:'#FFFFFF', primary:'#3858D6', field:'#f7f8fa', selected:'#E9EDFF', 'geometry-key':'#3858D6', 'grid-small':'#d8e1e8'
                    })) if (value(token).toLowerCase() !== expected.toLowerCase()) throw Error('Original mapping: ' + token);
                }
                const button = [...document.querySelectorAll('[data-slot=button]')].find(b => b.textContent.includes('Browse parts'));
                if (!button) throw Error('Real copied Button missing');
                const saved = button.className;
                const heights = [];
                try {
                    for (const [className, height] of [['h-control',40], ['h-control-compact',36], ['h-control-touch',44]]) {
                        button.className = saved.replace(/h-(?:11|control(?:-compact|-touch)?)/g, '') + ' ' + className;
                        button.getBoundingClientRect();
                        await Promise.all(button.getAnimations().map(animation => animation.finished));
                        const actual = button.getBoundingClientRect().height;
                        if (actual !== height) throw Error(className + ': ' + actual);
                        heights.push(actual);
                    }
                } finally { button.className = saved; }
                const font = root.dataset.theme === 'shadcn' ? 'Inter Variable' : 'Source Sans 3';
                const faces = await document.fonts.load('16px "' + font + '"');
                if (!faces.length) throw Error('Bundled font face missing: ' + font);
                if (!document.fonts.check('16px "' + font + '"')) throw Error('Font missing: ' + font);
                return {theme:root.dataset.theme, appearance:root.dataset.appearance, tokens:tokens.length, heights, font, background:value('background'), card:value('card'), geometry:value('geometry-key')};
            })()""")
            observations.append(result.strip())
            browser("screenshot", str((args.evidence / f"{theme}-{appearance}.png").resolve()))
    (args.evidence / "mounted.txt").write_text("\n".join(observations) + "\n")
    print("PASS: four Theme/Appearance combinations; tokens, desk/card, mounted canvas, legacy geometry aliases, control utilities and bundled fonts")


if __name__ == "__main__":
    main()
