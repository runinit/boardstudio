"""Advance a verified immutable candidate to selected-scope qualification, not acceptance."""
import hashlib
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


def changed_paths(root, source_commit, candidate_commit):
    """Return changed repository paths only when Git proves the source relationship."""
    if not re.fullmatch(r'[0-9a-f]{40}', str(source_commit)):
        return None
    try:
        git(root, 'merge-base', '--is-ancestor', source_commit, candidate_commit)
        output = git(root, 'diff', '--no-renames', '--name-only', '-z', source_commit, candidate_commit)
    except ValueError:
        return None
    return {path for path in output.split('\0') if path}


def _valid_repo_path(value):
    if (not isinstance(value, str) or not value or value.startswith("/")
            or "\\" in value or "\0" in value or re.match(r"^[A-Za-z]:", value)):
        return False
    parts = value.split("/")
    return all(part and part not in (".", "..") for part in parts)


def _repo_path(root, value):
    """Resolve a validated repository-relative path without traversing symlinks."""
    if not _valid_repo_path(value):
        return None
    path = Path(root)
    for part in Path(value).parts:
        path = path / part
        if path.is_symlink():
            return None
    return path


def _changed_under_root(changed, source_root):
    return changed == source_root or changed.startswith(source_root.rstrip("/") + "/")


def fixture_identities_match(journey, prior, root):
    """Require optional pinned fixture bytes to match this scope and its prior verdict."""
    current = journey.get('fixtures', [])
    previous = prior.get('fixtures', []) if isinstance(prior, dict) else []
    if not isinstance(current, list) or current != previous:
        return False
    seen = set()
    for fixture in current:
        if (not isinstance(fixture, dict) or set(fixture) != {'path', 'sha256'}
                or not _valid_repo_path(fixture['path'])
                or not isinstance(fixture['sha256'], str)
                or not re.fullmatch(r'[0-9a-f]{64}', fixture['sha256'])
                or fixture['path'] in seen):
            return False
        seen.add(fixture['path'])
        path = _repo_path(root, fixture['path'])
        if path is None:
            return False
        if not path.is_file():
            return False
        digest = hashlib.sha256()
        try:
            with path.open('rb') as stream:
                for chunk in iter(lambda: stream.read(1024 * 1024), b''):
                    digest.update(chunk)
        except OSError:
            return False
        if digest.hexdigest() != fixture['sha256']:
            return False
    return True


def reusable_journeys(scope_journeys, previous, previous_source, candidate_source, root):
    """Carry passed behavior only when its explicit source footprint stayed unchanged."""
    paths = changed_paths(root, previous_source, candidate_source)
    prior_by_id = {item.get('id'): item for item in previous if isinstance(item, dict)} \
        if isinstance(previous, list) else {}
    result = []
    for journey in clean_journeys(scope_journeys):
        prior = prior_by_id.get(journey.get('id'))
        footprint = journey.get('source_paths')
        source_roots = journey.get('source_roots', [])
        optional_paths = journey.get('optional_source_paths', [])
        complete_footprint = journey.get('source_paths_complete') is True
        valid_files = (isinstance(footprint, list)
                       and all(isinstance(path, str) and _repo_path(root, path) is not None
                               and _repo_path(root, path).is_file() for path in footprint))
        valid_roots = (isinstance(source_roots, list)
                       and all(isinstance(path, str) and _repo_path(root, path) is not None
                               and _repo_path(root, path).is_dir() for path in source_roots))
        valid_optional = (isinstance(optional_paths, list)
                          and all(isinstance(path, str) for path in optional_paths)
                          and len(optional_paths) == len(set(optional_paths))
                          and all(_repo_path(root, path) is not None
                                  and not _repo_path(root, path).exists()
                                  for path in optional_paths))
        covered_paths = set(footprint) if valid_files else set()
        covered_roots = source_roots if valid_roots else []
        covered_optional = set(optional_paths) if valid_optional else set()
        changed_covered = (paths is not None and any(
            changed in covered_paths or changed in covered_optional
            or any(_changed_under_root(changed, source_root)
                                            for source_root in covered_roots)
            for changed in paths))
        if (paths is None or not valid_files or not valid_roots or not valid_optional
                or not (footprint or source_roots or optional_paths) or not complete_footprint
                or changed_covered or not isinstance(prior, dict)
                or prior.get('state') != 'passed'
                or prior.get('scope') != journey.get('scope')
                or prior.get('spec') != journey.get('spec')
                or prior.get('source_paths') != footprint
                or prior.get('source_roots', []) != source_roots
                or prior.get('optional_source_paths', []) != optional_paths
                or prior.get('source_paths_complete') is not True
                or not fixture_identities_match(journey, prior, root)):
            result.append(dict(journey, state='pending'))
            continue
        evidence = prior.get('evidence')
        if not isinstance(evidence, str) or not evidence:
            result.append(dict(journey, state='pending'))
            continue
        evidence_path = Path(evidence)
        if evidence_path.is_absolute() or '..' in evidence_path.parts or not (Path(root) / evidence_path).is_file():
            result.append(dict(journey, state='pending'))
            continue
        # The old receipt remains an explicit pointer; the verdict is marked as
        # reused so candidate-scoped and durable evidence remain distinguishable.
        result.append(dict(journey, **{key: prior[key] for key in JOURNEY_RESULT_FIELDS if key in prior},
                           reuse='unchanged_scoped_source'))
    return result


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
        prior_source = state.get('source_commit', '')
        next_state.update(candidate_build_id=proof['build_id'], source_commit=proof['source_commit'],
                          journeys=reusable_journeys(scope.get('journeys', []), state.get('journeys', []),
                                                     prior_source, proof['source_commit'], root))
        next_state.pop('review', None)
        next_state.pop('started_at', None)
    if not unmet:
        if candidate_changed:
            next_state.update(candidate_build_id=proof['build_id'], source_commit=proof['source_commit'],
                              started_at=datetime.now(timezone.utc).isoformat())
            review = next_state.get('review')
            if not (isinstance(review, dict) and review.get('state') == 'completed'):
                next_state['review'] = {'profile':'sol-review', 'state':'pending'}
        else:
            next_state.update(candidate_build_id=proof['build_id'], source_commit=proof['source_commit'],
                              started_at=datetime.now(timezone.utc).isoformat(),
                              review={'profile':'sol-review', 'state':'pending'},
                              journeys=clean_journeys(scope.get('journeys', []), pending=True))
    if next_state == state:
        return False
    progress['qualification'] = next_state
    return True
