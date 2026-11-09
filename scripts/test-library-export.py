#!/usr/bin/env python3
"""Ticket 023: trusted role/name workflows on the packaged Library and Export page."""
import argparse
import atexit
import json
import re
from pathlib import Path
import subprocess
from uuid import uuid4


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('url')
    parser.add_argument('--evidence', type=Path, required=True)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    session = 'library-export-023-' + uuid4().hex

    def browser(*command):
        result = subprocess.run(['agent-browser', '--session', session, *command],
                                text=True, capture_output=True, timeout=90)
        print(command, result.stdout, result.stderr, flush=True)
        assert result.returncode == 0, result.stdout + result.stderr
        return result.stdout

    def role_ref(role, name):
        snapshot = json.loads(browser('snapshot', '--json', '-i'))['data']
        refs = re.findall(r'^\s*- ' + re.escape(role) + ' "' + re.escape(name) + r'" \[[^\]\n]*\bref=(e\d+)', snapshot['snapshot'], re.MULTILINE)
        assert len(refs) == 1, (role, name, snapshot)
        return '@' + refs[0]

    def styles(role, name):
        return json.loads(browser('get', 'styles', role_ref(role, name), '--json'))['data']['styles']

    def action(role, name, verb='click', value=None):
        if verb in ('focus', 'download'):
            browser(verb, role_ref(role, name), *([value] if value is not None else []))
            return
        command = ['find', 'role', role, verb]
        if value is not None:
            command.append(value)
        browser(*command, '--name', name, '--exact')

    def check(script):
        browser('eval', '(() => { ' + script + '; return true; })()')

    def touch_target(role, name):
        action(role, name, 'focus')
        check("const e = document.activeElement, r = e.getBoundingClientRect(), c = getComputedStyle(e); if (r.width < 44 || r.height < 44) throw Error('Touch target below 44px: ' + JSON.stringify({width:r.width,height:r.height,minHeight:c.minHeight,heightStyle:c.height,scale:c.scale,transform:c.transform,active:e.matches(':active'),sheets:[...document.querySelectorAll('link[rel=stylesheet]')].map(l=>({href:l.href,ready:!!l.sheet}))}) + e.outerHTML);")

    atexit.register(lambda: subprocess.run(['agent-browser', '--session', session, 'close'],
                                         text=True, capture_output=True, timeout=30))
    try:
        browser('set', 'viewport', '1440', '1000')
        browser('open', args.url)
        browser('wait', '--text', 'Create new')
        browser('wait', '--fn', "!!document.querySelector('link[rel=stylesheet][href$=\"tailwind.css\"]')?.sheet && [...document.querySelectorAll('link[rel=stylesheet]')].every(link => !!link.sheet)")
        browser('eval', '(async () => { await document.fonts.ready; return true; })()')
        check("if (!matchMedia('(pointer:coarse)').matches) throw Error('Run this floor workflow with --blink-settings=primaryPointerType=2');")
        for name in ('Open demo', 'Import project', 'Create new keyboard'):
            touch_target('button', name)
        browser('screenshot', str((args.evidence / 'library-start.png').resolve()))
        action('button', 'Create new keyboard')
        browser('wait', '--text', 'Continue to layout')
        touch_target('textbox', 'Project name')
        for name in ('Project & hardware', 'Layout & assemblies', 'Controller & wiring', 'Case (optional)', 'Hide guide'):
            touch_target('button', name)
        action('button', 'Project & hardware', 'focus')
        browser('press', 'End')
        check("if (document.activeElement.getAttribute('aria-label') !== 'Case (optional)') throw Error('Accordion End must reach Case');")
        browser('press', 'Space')
        browser('wait', '--fn', "document.querySelector('[aria-label=\"Case (optional)\"]').getAttribute('aria-current') === 'step'")
        browser('wait', '--fn', "document.querySelector('#m1-setup-panel-3').getAttribute('aria-hidden') === 'false' && !document.querySelector('[aria-label=\"Project setup guide\"]').getAnimations({subtree:true}).some(a => a.playState === 'running')")
        action('button', 'Open full case settings')
        browser('wait', '[aria-label="Full case settings"]')
        touch_target('button', 'Back to setup guide')
        browser('press', 'Enter')
        browser('wait', '[aria-label="Project setup guide"]')
        action('button', 'Case (optional)', 'focus')
        browser('press', 'Home')
        browser('press', 'Enter')
        browser('wait', '--fn', "document.querySelector('[aria-label=\"Project & hardware\"]').getAttribute('aria-current') === 'step'")
        check("if ([...document.querySelectorAll('[aria-label=\"Project setup guide\"] [data-slot=accordion-trigger]')].some(e=>e.disabled)) throw Error('Guide stages must remain freely navigable');")
        browser('wait', '--fn', "!document.querySelector('[aria-label=\"Project setup guide\"]').getAnimations({subtree:true}).some(a => a.playState === 'running')")
        check("const trigger = document.querySelector('[aria-label=\"Project & hardware\"]'), p = document.getElementById(trigger.getAttribute('aria-controls')), c = p.firstElementChild; if (p.getAttribute('aria-hidden') !== 'false' || c.getBoundingClientRect().bottom > p.getBoundingClientRect().bottom + 1) throw Error('Open guide content must fit its panel');")
        browser('screenshot', str((args.evidence / 'setup-guide.png').resolve()))
        browser('set', 'viewport', '390', '1000')
        touch_target('textbox', 'Project name')
        check("const g = document.querySelector('[aria-label=\"Project setup guide\"]'); if (g.scrollWidth > g.clientWidth + 1) throw Error('Guide must fit the narrow drawer');")
        browser('screenshot', str((args.evidence / 'setup-guide-390.png').resolve()))
        browser('set', 'viewport', '1440', '1000')
        action('button', 'Continue to layout')
        action('button', 'Hide guide')
        action('button', 'Cancel')
        browser('press', 'Control+k')
        action('combobox', 'Search commands and objects', 'fill', 'Browse projects')
        action('option', 'Browse projects')
        browser('wait', '--text', 'Your saved keyboards')
        action('button', 'Delete Untitled keyboard', 'focus')
        check('window.cardDeleteOpener = document.activeElement')
        check("const b = cardDeleteOpener.getBoundingClientRect(), c = cardDeleteOpener.closest('[data-slot=card]').getBoundingClientRect(); if (b.top < c.top || b.bottom > c.bottom || b.right > c.right) throw Error('Delete target must stay inside its keyboard Card');")
        browser('wait', '[role=tooltip]')
        check("const id = cardDeleteOpener.getAttribute('aria-describedby'); if (!id || !document.getElementById(id)) throw Error('Delete Tooltip must describe its keyboard trigger');")
        browser('press', 'Escape')
        browser('wait', '--fn', '!document.querySelector("[role=tooltip]")')
        check("if (document.activeElement !== cardDeleteOpener) throw Error('Tooltip Escape must retain focus');")
        browser('press', 'Enter')
        browser('wait', '[role=alertdialog]')
        check("if (document.activeElement.textContent.trim() !== 'Cancel' || cardDeleteOpener.getAttribute('aria-haspopup') !== 'dialog' || cardDeleteOpener.getAttribute('aria-expanded') !== 'true') throw Error('Card deletion must start safely and expose popup state');")
        browser('press', 'Escape')
        browser('wait', '--fn', "!document.querySelector('[role=alertdialog]')")
        check("if (document.activeElement !== cardDeleteOpener || cardDeleteOpener.getAttribute('aria-expanded') !== 'false') throw Error('Escape must restore the exact card opener');")
        action('button', 'Preview Untitled keyboard')
        browser('wait', '--text', 'Delete keyboard')
        action('button', 'Delete keyboard', 'focus')
        check('window.deleteOpener = document.activeElement')
        browser('press', 'Enter')
        browser('wait', '[role=alertdialog]')
        check('''
            window.deleteIdentity = document.querySelector('[aria-label="Delete Untitled keyboard"]').getAttribute('data-project-id');
            window.deleteDialog = document.querySelector('[role=alertdialog]');
            if (deleteDialog.getAttribute('aria-modal') !== 'true') throw Error('Delete must be modal');
            for (const attr of ['aria-labelledby', 'aria-describedby']) {
                if (!document.getElementById(deleteDialog.getAttribute(attr))) throw Error(attr + ' must resolve');
            }
            if (document.activeElement.textContent.trim() !== 'Cancel') throw Error('Cancel must be the safe initial action');
        ''')
        browser('press', 'Shift+Tab')
        check("if (!deleteDialog.contains(document.activeElement) || document.activeElement.textContent.trim() !== 'Delete keyboard') throw Error('Shift Tab must wrap inside confirmation');")
        browser('press', 'Tab')
        check("if (document.activeElement.textContent.trim() !== 'Cancel') throw Error('Tab must wrap to Cancel');")
        browser('press', 'Control+k')
        browser('wait', '[role=combobox][aria-label="Search commands and objects"]')
        check("if (!document.querySelector('#command-palette').matches(':modal')) throw Error('Palette must retain its native modal host over deletion');")
        browser('press', 'Shift+Tab')
        check("if (!document.querySelector('#command-palette').contains(document.activeElement)) throw Error('Palette must trap focus over deletion');")
        browser('press', 'Escape')
        browser('wait', '--fn', "!document.querySelector('#command-palette[open]')")
        check("if (!deleteDialog.isConnected || document.activeElement.textContent.trim() !== 'Cancel') throw Error('Palette Escape must preserve deletion and restore Cancel');")
        action('button', 'Cancel')
        browser('wait', '--fn', "!document.querySelector('[role=alertdialog]')")
        check("if (document.activeElement !== deleteOpener || document.querySelector('[aria-label=\"Delete Untitled keyboard\"]').getAttribute('data-project-id') !== deleteIdentity) throw Error('Cancel must preserve identity and restore the actual sidebar opener');")
        action('button', 'Delete keyboard')
        browser('wait', '[role=alertdialog]')
        browser('press', 'Escape')
        browser('wait', '--fn', "!document.querySelector('[role=alertdialog]')")
        check("if (document.activeElement !== deleteOpener || document.querySelector('[aria-label=\"Delete Untitled keyboard\"]').getAttribute('data-project-id') !== deleteIdentity) throw Error('Escape must preserve identity and restore the actual opener');")
        action('button', 'Delete keyboard')
        browser('wait', '[role=alertdialog]')
        browser('screenshot', str((args.evidence / 'delete-confirmation.png').resolve()))
        # Scoped role/name lookup disambiguates the sidebar action and confirmation.
        snapshot = json.loads(browser('snapshot', '--json', '-i', '--selector', '[role=alertdialog]'))['data']
        refs = re.findall(r'^\s*- button "Delete keyboard" \[[^\]\n]*\bref=(e\d+)', snapshot['snapshot'], re.MULTILINE)
        assert len(refs) == 1, snapshot
        browser('click', '@' + refs[0])
        browser('wait', '--fn', "!document.querySelector('[role=alertdialog]') && ![...document.querySelectorAll('[data-project-id]')].some(e => e.getAttribute('data-project-id') === deleteIdentity)")
        check("if (!document.querySelector('[aria-label=\"Delete Untitled keyboard\"]')?.getAttribute('data-project-id')) throw Error('Deleting the last project must retain its durable replacement');")
        action('button', 'Preview Untitled keyboard')
        action('button', 'Open keyboard')
        action('button', 'Export', 'focus')
        browser('wait', '--fn', "(() => { const e = document.activeElement, r = e.getBoundingClientRect(); return e.contains(document.elementFromPoint(r.x + r.width/2, r.y + r.height/2)); })()")
        action('button', 'Export')
        browser('wait', '[aria-label="Export package"]')
        export_snapshot = browser('snapshot')
        assert not re.search(r'^\s*- heading "(?:Objects|Inspector)"', export_snapshot, re.MULTILINE), 'Export is a separate page; workspace docks must stay hidden'
        check('''
            const footer = document.querySelector('footer[aria-label="Canvas status"]');
            if (!footer || getComputedStyle(footer).display !== 'none') throw Error('Export must hide the canvas status footer');
            for (const name of ['Export KiCad board', 'Export ZMK firmware']) {
                const button = document.querySelector('[aria-label="' + name + '"]');
                if (!button.disabled) throw Error(name + ' must remain blocked for an empty design');
                if (!button.closest('[data-export-kind]').textContent.includes('Review the layout and resolve controller wiring in PCB.')) throw Error('Export blocker must stay inline');
            }
        ''')
        for role, name in [('checkbox', 'Embed used models'), ('button', 'Download .boardstudio project'), ('button', 'Review wiring'), ('button', 'Review case')]:
            touch_target(role, name)
        action('checkbox', 'Embed used models', 'focus')
        check('window.embedBefore = document.activeElement.getAttribute("aria-checked")')
        browser('press', 'Space')
        browser('wait', '--fn', 'document.activeElement.getAttribute("aria-checked") !== embedBefore')
        action('button', 'Download .boardstudio project', 'download', str((args.evidence / 'project-copy.boardstudio').resolve()))
        assert (args.evidence / 'project-copy.boardstudio').stat().st_size > 0
        browser('wait', '--text', 'Project copy downloaded.')
        browser('screenshot', str((args.evidence / 'export-outcome.png').resolve()))
        download = styles('button', 'Download .boardstudio project')
        review = styles('button', 'Review wiring')
        assert download['background-color'] == review['background-color'], 'Portable copy must retain the copied Secondary fill'
        for theme in ('original', 'shadcn'):
            browser('set', 'viewport', '1440', '1000')
            browser('wait', '[aria-label="Settings"]')
            action('button', 'Settings')
            action('menuitemradio', 'Original' if theme == 'original' else 'shadcn')
            for appearance in ('light', 'dark'):
                browser('set', 'viewport', '1440', '1000')
                browser('wait', '[aria-label="Settings"]')
                action('button', 'Settings')
                action('menuitemradio', appearance.title())
                browser('wait', '--fn', f'document.documentElement.dataset.theme === "{theme}" && document.documentElement.dataset.appearance === "{appearance}"')
                browser('wait', '--fn', "!document.querySelector('[role=menu][data-state=open]') && ![...document.querySelectorAll('[data-slot=dropdown-menu-content]')].some(menu => menu.getAnimations({subtree:true}).some(a => a.playState === 'running'))")
                for width in (1440, 390):
                    browser('set', 'viewport', str(width), '1000')
                    check('''
                        const root = document.querySelector('[aria-label="Export package"]');
                        if (root.scrollWidth > root.clientWidth + 1) throw Error('Export must fit the viewport');
                    ''')
                    assert styles('button', 'Review wiring')['border-top-width'] == '0px', 'Secondary buttons must remain borderless'
                    browser('screenshot', str((args.evidence / f'export-{theme}-{appearance}-{width}.png').resolve()))
        browser('set', 'viewport', '1440', '1000')
        browser('press', 'Escape')
        browser('wait', '--fn', "!document.querySelector('[aria-label=\"Export package\"]')")
        browser('press', 'Control+k')
        action('combobox', 'Search commands and objects', 'fill', 'Browse projects')
        action('option', 'Browse projects')
        browser('wait', '--text', 'Your saved keyboards')
        for width in (1440, 390):
            browser('set', 'viewport', str(width), '1000')
            touch_target('searchbox', 'Search saved keyboards')
            touch_target('button', 'Delete Untitled keyboard')
            browser('wait', '[role=tooltip]')
            check("const t = document.querySelector('[role=tooltip]').getBoundingClientRect(); if (t.left < 0 || t.right > innerWidth) throw Error('Card Tooltip must fit the viewport');")
            check("const l = document.querySelector('[aria-label=\"Your keyboards\"]'); if (l.scrollWidth > l.clientWidth + 1) throw Error('Library must fit the viewport');")
            browser('screenshot', str((args.evidence / f'library-shadcn-dark-{width}.png').resolve()))
        check("if (document.body.textContent.includes('App panicked!')) throw Error('Library navigation must not leave a dropped owner task');")
        print('PASS: Library deletion identity/safe focus/Ctrl+K; free guide keyboard stages; Export inline blockers, Checkbox, download outcome and both Themes/Appearances', flush=True)
    except BaseException:
        for command in [('snapshot',), ('console',), ('errors',),
                        ('screenshot', str((args.evidence / 'failure.png').resolve()))]:
            result = subprocess.run(['agent-browser', '--session', session, *command],
                                    text=True, capture_output=True, timeout=30)
            print('DIAGNOSTIC', command, result.stdout, result.stderr, flush=True)
        raise


if __name__ == '__main__':
    main()
