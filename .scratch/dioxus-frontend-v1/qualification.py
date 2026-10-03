"""Advance a verified immutable candidate to selected-scope qualification, not acceptance."""
from datetime import datetime, timezone
from pathlib import Path
import re
import subprocess

PAGE_CHECK = ['cargo', 'check', '--manifest-path', 'web/Cargo.toml', '--locked',
              '--target', 'wasm32-unknown-unknown', '--no-default-features',
              '--features', 'page', '--bin', 'boardstudio-web']
JOURNEY_RESULT_FIELDS = {'state', 'verified', 'remaining', 'evidence', 'limitation',
                         'result', 'verdict', 'completed_at'}


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


def selected_scope(state):
    requested = state.get('next_scope')
    active = state.get('active_scope')
    if isinstance(requested, dict) and requested != active:
        return requested, True, False
    if isinstance(active, dict):
        return active, False, False
    # Older run records pin Layout directly. Keep interpreting that shape until
    # the coordinator explicitly selects a functional next_scope.
    return {
        'id': 'layout-ready',
        'source_commit': state.get('layout_source_commit', ''),
        'streams': ['Layout'],
        'journeys': state.get('journeys', []),
    }, False, True


def packet_blocks_scope(packet, streams):
    packet_stream = str(packet.get('stream', '')).strip().casefold()
    for stream in streams:
        name = str(stream).strip().casefold()
        if packet_stream == name or (name == 'layout' and packet_stream.startswith('layout')):
            return True
    return False


def clean_journeys(journeys, pending=False):
    cleaned = []
    if not isinstance(journeys, list):
        return cleaned
    for journey in journeys:
        if not isinstance(journey, dict):
            continue
        item = {key: value for key, value in journey.items()
                if key not in JOURNEY_RESULT_FIELDS}
        if pending:
            item['state'] = 'pending'
        cleaned.append(item)
    return cleaned


def refresh(progress, proof, provenance, root):
    """Called only after package/provenance/live assets have been verified."""
    state = progress.get('qualification')
    if not state:
        return False  # No milestone has been selected in this run.
    scope, starting_scope, legacy_layout = selected_scope(state)
    if (not starting_scope and state.get('candidate_build_id') == proof['build_id'] and
            state.get('phase') in ('qualifying', 'qualified', 'repairing')):
        return False  # Keep frozen scope/results while unrelated source queues advance.
    unmet = []
    source = scope.get('source_commit', '')
    label = 'Layout' if legacy_layout else str(scope.get('label', scope.get('id', 'Functional')))
    try:
        if not re.fullmatch(r'[0-9a-f]{40}', source):
            raise ValueError('missing pin')
        git(root, 'merge-base', '--is-ancestor', source, proof['source_commit'])
    except ValueError:
        unmet.append(f'{label} source scope is not proven integrated into this candidate')
    streams = scope.get('streams', ['Layout'] if legacy_layout else [])
    if not legacy_layout and (not isinstance(scope.get('id'), str) or not scope['id'].strip()):
        unmet.append(f'{label} scope ID is missing')
    valid_streams = (isinstance(streams, list) and bool(streams)
                     and all(isinstance(stream, str) and stream.strip() for stream in streams))
    if not valid_streams and not legacy_layout:
        unmet.append(f'{label} stream scope is invalid')
    if not isinstance(streams, list):
        streams = []
    streams = [stream for stream in streams if isinstance(stream, str) and stream.strip()]
    journeys = scope.get('journeys', [])
    if not legacy_layout:
        ids = [journey.get('id') for journey in journeys if isinstance(journey, dict)] \
            if isinstance(journeys, list) else []
        if (not isinstance(journeys, list) or not journeys or len(ids) != len(journeys)
                or any(not isinstance(journey_id, str) or not journey_id.strip() for journey_id in ids)
                or len(set(ids)) != len(ids)):
            unmet.append(f'{label} journey scope must have unique nonempty journey IDs')
    blocked = [packet for packet in progress.get('pending_join_packets', [])
               if packet_blocks_scope(packet, streams)]
    if blocked:
        names = ', '.join(dict.fromkeys(str(packet.get('stream', '')) for packet in blocked))
        unmet.append(f'{names or label} still has a queued implementation packet')
    if not compile_satisfied(proof, provenance, root):
        unmet.append('A successful matching combined page compiler check is missing')
    if starting_scope:
        history = list(state.get('history', []))
        prior_scope = {key: value for key, value in state.items() if key != 'history'}
        history.append(prior_scope)
        next_state = {
            'trigger': scope.get('trigger', 'functional_scope'),
            'active_scope': dict(scope) | {'journeys': clean_journeys(scope.get('journeys', []))},
            'history': history,
            'phase': 'integrating' if unmet else 'qualifying',
            'unmet_start_conditions': unmet,
            'candidate_build_id': proof['build_id'],
            'source_commit': proof['source_commit'],
            'journeys': clean_journeys(scope.get('journeys', []), pending=True),
        }
        if 'authority' in state:
            next_state['authority'] = state['authority']
    else:
        next_state = dict(state, phase='integrating' if unmet else 'qualifying',
                          unmet_start_conditions=unmet)
    candidate_changed = (not starting_scope and state.get('candidate_build_id')
                         and state['candidate_build_id'] != proof['build_id'])
    if candidate_changed:
        # Results belong to the candidate that produced them. Archive them even
        # when the replacement candidate is still waiting on a scoped join.
        next_state.setdefault('history', [])
        next_state['history'] = [*next_state['history'], {
            key: state[key] for key in ('candidate_build_id', 'source_commit', 'phase', 'review', 'journeys')
            if key in state
        }]
        next_state.update(candidate_build_id=proof['build_id'], source_commit=proof['source_commit'],
                          journeys=clean_journeys(scope.get('journeys', []), pending=True))
        next_state.pop('review', None)
        next_state.pop('started_at', None)
    if not unmet:
        next_state.update(candidate_build_id=proof['build_id'], source_commit=proof['source_commit'],
                          started_at=datetime.now(timezone.utc).isoformat(),
                          review={'profile':'sol-review', 'state':'pending'},
                          journeys=clean_journeys(scope.get('journeys', []), pending=True))
    if next_state == state:
        return False
    progress['qualification'] = next_state
    return True
