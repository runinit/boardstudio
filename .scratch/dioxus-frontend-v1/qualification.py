"""Advance a verified immutable candidate to Layout qualification, not acceptance."""
from datetime import datetime, timezone
from pathlib import Path
import re
import subprocess

PAGE_CHECK = ['cargo', 'check', '--manifest-path', 'web/Cargo.toml', '--locked',
              '--target', 'wasm32-unknown-unknown', '--no-default-features',
              '--features', 'page', '--bin', 'boardstudio-web']


def git(root, *args):
    result = subprocess.run(['git', '-C', str(root), *args], capture_output=True, text=True)
    if result.returncode:
        raise ValueError(result.stderr.strip() or 'Git source relationship is unproven')
    return result.stdout.strip()


def compile_satisfied(proof, provenance, root):
    if any(command.get('argv') == PAGE_CHECK and command.get('exit') == 0
           for command in provenance['commands']):
        return True
    # Older retained candidates recorded the same check outside packaging. Reuse it
    # only when its source differs by documentation; future builds include the check.
    check = proof.get('source_compile', {})
    if check.get('exit_code') != 0 or not check.get('source_commit'):
        return False
    log = Path(check.get('log', ''))
    if not log.is_file() or 'Finished' not in log.read_text():
        return False
    try:
        paths = git(root, 'diff', '--name-only', check['source_commit'], proof['source_commit']).splitlines()
    except ValueError:
        return False
    return all(path.startswith(('docs/', '.scratch/')) or path in
               ('CONSTRAINTS.md', 'AGENTS.md', 'README.md', 'PLAN.md', 'TODO.md') for path in paths)


def refresh(progress, proof, provenance, root):
    """Called only after package/provenance/live assets have been verified."""
    state = progress.get('qualification')
    if not state:
        return False  # No milestone has been selected in this run.
    if (state.get('candidate_build_id') == proof['build_id'] and
            state.get('phase') in ('qualifying', 'qualified', 'repairing')):
        return False  # Keep the frozen scope/results while other source queues advance.
    unmet = []
    source = state.get('layout_source_commit', '')
    try:
        if not re.fullmatch(r'[0-9a-f]{40}', source):
            raise ValueError('missing pin')
        git(root, 'merge-base', '--is-ancestor', source, proof['source_commit'])
    except ValueError:
        unmet.append('Layout source scope is not proven integrated into this candidate')
    if any(packet.get('stream', '').lower().startswith('layout')
           for packet in progress.get('pending_join_packets', [])):
        unmet.append('Layout still has a queued implementation packet')
    if not compile_satisfied(proof, provenance, root):
        unmet.append('A successful matching combined page compiler check is missing')
    next_state = dict(state, phase='integrating' if unmet else 'qualifying',
                      unmet_start_conditions=unmet)
    if not unmet:
        if state.get('candidate_build_id') and state['candidate_build_id'] != proof['build_id']:
            # Preserve results for the previous scope without copying its receipts.
            next_state.setdefault('history', [])
            next_state['history'] = [*next_state['history'], {
                key: state[key] for key in ('candidate_build_id', 'source_commit', 'phase', 'review', 'journeys')
                if key in state
            }]
        next_state.update(candidate_build_id=proof['build_id'], source_commit=proof['source_commit'],
                          started_at=datetime.now(timezone.utc).isoformat(),
                          review={'profile':'sol-review', 'state':'pending'},
                          journeys=[dict(journey, state='pending') for journey in state.get('journeys', [])])
    if next_state == state:
        return False
    progress['qualification'] = next_state
    return True
