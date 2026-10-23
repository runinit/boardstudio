#!/usr/bin/env python3
"""Ticket 023: trusted role/name workflows on the packaged Library and Export page."""
import argparse
import atexit
import json
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

    def action(role, name, verb='click', value=None):
        command = ['find', 'role', role, verb]
        if value is not None:
            command.append(value)
        browser(*command, '--name', name, '--exact')

    def check(script):
        browser('eval', '(() => { ' + script + '; return true; })()')

    atexit.register(lambda: subprocess.run(['agent-browser', '--session', session, 'close'],
                                         text=True, capture_output=True, timeout=30))
    try:
        browser('set', 'viewport', '1440', '1000')
        browser('open', args.url)
        browser('wait', '--text', 'Create new')
        action('button', 'Create new keyboard')
        browser('wait', '--text', 'Continue to layout')
        action('button', 'Continue to layout')
        action('button', 'Hide guide')
        action('button', 'Cancel')
        browser('press', 'Control+k')
        action('combobox', 'Search commands and objects', 'fill', 'Browse projects')
        action('option', 'Browse projects')
        browser('wait', '--text', 'Your saved keyboards')
        action('button', 'Preview Untitled keyboard')
        browser('wait', '--text', 'Delete keyboard')
        action('button', 'Delete keyboard')
        browser('wait', '[role=alertdialog]')
        check('''
            window.deleteOpener = document.querySelector('[aria-label="Keyboard details and starting guidance"] [data-slot=button][class*=destructive]');
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
        check("if (document.activeElement.textContent.trim() !== 'Delete keyboard' || document.querySelector('[aria-label=\"Delete Untitled keyboard\"]').getAttribute('data-project-id') !== deleteIdentity) throw Error('Cancel must preserve identity and restore the actual sidebar opener');")
        action('button', 'Delete keyboard')
        browser('wait', '[role=alertdialog]')
        browser('press', 'Escape')
        browser('wait', '--fn', "!document.querySelector('[role=alertdialog]')")
        check("if (document.activeElement.textContent.trim() !== 'Delete keyboard' || document.querySelector('[aria-label=\"Delete Untitled keyboard\"]').getAttribute('data-project-id') !== deleteIdentity) throw Error('Escape must preserve identity and restore the actual opener');")
        action('button', 'Delete keyboard')
        browser('wait', '[role=alertdialog]')
        browser('screenshot', str((args.evidence / 'delete-confirmation.png').resolve()))
        # Scoped role/name lookup disambiguates the sidebar action and confirmation.
        snapshot = json.loads(browser('snapshot', '--json', '-i', '--selector', '[role=alertdialog]'))['data']
        refs = [ref for ref, item in snapshot['refs'].items()
                if item.get('role') == 'button' and item.get('name') == 'Delete keyboard']
        assert len(refs) == 1, snapshot
        browser('click', '@' + refs[0])
        browser('wait', '--fn', "!document.querySelector('[role=alertdialog]') && ![...document.querySelectorAll('[data-project-id]')].some(e => e.getAttribute('data-project-id') === deleteIdentity)")
        check("if (!document.querySelector('[aria-label=\"Delete Untitled keyboard\"]')?.getAttribute('data-project-id')) throw Error('Deleting the last project must retain its durable replacement');")
        print('PASS: Library Alert Dialog identity, safe action, trusted keyboard, focus and Ctrl+K integration', flush=True)
    except BaseException:
        for command in [('snapshot',), ('console',), ('errors',),
                        ('screenshot', str((args.evidence / 'failure.png').resolve()))]:
            result = subprocess.run(['agent-browser', '--session', session, *command],
                                    text=True, capture_output=True, timeout=30)
            print('DIAGNOSTIC', command, result.stdout, result.stderr, flush=True)
        raise


if __name__ == '__main__':
    main()
