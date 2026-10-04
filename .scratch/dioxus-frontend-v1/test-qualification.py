"""Checks for the Layout-ready phase boundary; no application tests."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

path = Path(__file__).with_name('qualification.py')
spec = importlib.util.spec_from_file_location('qualification', path)
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)

class QualificationTests(unittest.TestCase):
    def setUp(self):
        self.progress = {'qualification': {'layout_source_commit': 'a'*40, 'phase': 'integrating'},
                         'pending_join_packets': [], 'parent_counts': {'accepted': 10}}
        self.proof = {'build_id': 'candidate', 'source_commit': 'b'*40}
        self.provenance = {'commands': [{'argv': q.PAGE_CHECK, 'exit': 0, 'started': '2026-10-03T12:00:00+00:00'}]}
    def refresh(self):
        with patch.object(q, 'git', return_value=''):
            return q.refresh(self.progress, self.proof, self.provenance, Path('/unused'))
    def test_layout_ready_starts_even_with_other_stream_pending(self):
        self.progress['pending_join_packets'] = [{'stream':'PCB'}]
        self.assertTrue(self.refresh())
        self.assertEqual(self.progress['qualification']['phase'], 'qualifying')
        self.assertEqual(self.progress['qualification']['review']['profile'], 'sol-review')
        self.assertEqual(self.progress['parent_counts']['accepted'], 10)
        self.assertFalse(self.refresh())
    def test_layout_packet_and_missing_compile_hold_gate(self):
        self.progress['pending_join_packets'] = [{'stream':'Layout'}]
        self.provenance['commands'] = []
        self.refresh()
        self.assertEqual(self.progress['qualification']['phase'], 'integrating')
        self.assertEqual(len(self.progress['qualification']['unmet_start_conditions']), 2)
    def test_failed_preflight_is_not_compile_evidence(self):
        self.provenance['commands'][0]['exit'] = 101
        self.refresh()
        self.assertEqual(self.progress['qualification']['phase'], 'integrating')
    def test_unintegrated_layout_pin_holds_gate(self):
        with patch.object(q, 'git', side_effect=ValueError('not ancestor')):
            q.refresh(self.progress, self.proof, self.provenance, Path('/unused'))
        self.assertIn('Layout source', self.progress['qualification']['unmet_start_conditions'][0])
    def test_legacy_compile_reused_only_with_unchanged_source(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory)/'compile.log'
            log.write_text('Finished dev profile')
            self.proof['source_compile'] = {'source_commit':'c'*40,'exit_code':0,'log':str(log)}
            self.provenance['commands'] = []
            with patch.object(q, 'git', return_value='docs/run.json\n'):
                self.assertTrue(q.compile_satisfied(self.proof,self.provenance,Path(directory)))
            with patch.object(q, 'git', return_value='web/src/main.rs\n'):
                self.assertFalse(q.compile_satisfied(self.proof,self.provenance,Path(directory)))
    def test_ongoing_frozen_candidate_not_restarted_by_new_work(self):
        self.refresh()
        self.progress['qualification']['journeys']=[{'id':'outline','state':'passed'}]
        self.progress['pending_join_packets']=[{'stream':'Layout'}]
        self.assertFalse(self.refresh())
        self.assertEqual(self.progress['qualification']['journeys'][0]['state'],'passed')
    def test_functional_scope_uses_its_pin_and_ignores_layout_packet(self):
        state = self.progress['qualification']
        state.update(candidate_build_id='layout-candidate', source_commit='a'*40,
                     phase='repairing', review={'state':'completed'},
                     journeys=[{'id':'old-layout', 'state':'partial', 'verified':'retained'}],
                     history=[{'candidate_build_id':'earlier-layout'}],
                     next_scope={'id':'keymap-functional', 'source_commit':'c'*40,
                                 'streams':['Keymap'],
                                 'journeys':[{'id':'encoder-edit', 'scope':'Edit and persist encoder',
                                              'state':'partial', 'verified':'stale result',
                                              'remaining':'stale gap', 'evidence':'old receipt'}]})
        self.progress['pending_join_packets']=[{'stream':'Layout'}]
        with patch.object(q, 'git', return_value='') as git:
            self.assertTrue(q.refresh(self.progress, self.proof, self.provenance, Path('/unused')))
        git.assert_called_with(Path('/unused'), 'merge-base', '--is-ancestor', 'c'*40, 'b'*40)
        active = self.progress['qualification']
        self.assertEqual(active['phase'], 'qualifying')
        self.assertEqual(active['active_scope']['id'], 'keymap-functional')
        self.assertEqual(active['journeys'], [{'id':'encoder-edit', 'scope':'Edit and persist encoder', 'state':'pending'}])
        self.assertEqual(active['history'][0], {'candidate_build_id':'earlier-layout'})
        self.assertEqual(active['history'][1]['journeys'], [{'id':'old-layout', 'state':'partial', 'verified':'retained'}])
        self.assertEqual(active['history'][1]['review'], {'state':'completed'})
        self.assertEqual(active['active_scope']['journeys'], [
            {'id':'encoder-edit', 'scope':'Edit and persist encoder'}])
        active['journeys'][0].update(state='passed', verified='new candidate result')
        self.assertFalse(q.refresh(self.progress, self.proof, self.provenance, Path('/unused')))
        self.assertEqual(active['journeys'][0]['state'], 'passed')

    def test_functional_scope_waits_only_for_its_stream_packets(self):
        state = self.progress['qualification']
        state['next_scope']={'id':'keymap-functional', 'source_commit':'c'*40,
                             'streams':['Keymap'], 'journeys':[{'id':'encoder-edit'}]}
        self.progress['pending_join_packets']=[{'stream':'Keymap'}]
        self.refresh()
        active = self.progress['qualification']
        self.assertEqual(active['phase'], 'integrating')
        self.assertTrue(any('Keymap' in reason for reason in active['unmet_start_conditions']))

    def test_new_candidate_resets_active_functional_journey_results(self):
        state = self.progress['qualification']
        scope = {'id':'keymap-functional', 'source_commit':'c'*40,
                 'streams':['Keymap'],
                 'journeys':[{'id':'encoder-edit', 'state':'partial',
                              'verified':'old candidate result', 'remaining':'old gap',
                              'evidence':'old-candidate.json'}]}
        state['next_scope'] = scope
        self.refresh()
        state = self.progress['qualification']
        state['journeys'][0].update(state='passed', verified='candidate result', evidence='receipt.json')
        self.proof = {'build_id':'candidate-2', 'source_commit':'d'*40}
        with patch.object(q, 'git', return_value=''):
            self.assertTrue(q.refresh(self.progress, self.proof, self.provenance, Path('/unused')))
        active = self.progress['qualification']
        self.assertEqual(active['journeys'], [{'id':'encoder-edit', 'state':'pending'}])
        self.assertEqual(active['history'][-1]['journeys'][0]['state'], 'passed')

    def test_new_candidate_with_unmet_scope_gate_still_archives_old_journey_verdict(self):
        self.progress['qualification']['next_scope']={
            'id':'keymap-functional', 'source_commit':'c'*40, 'streams':['Keymap'],
            'journeys':[{'id':'encoder-edit'}],
        }
        self.refresh()
        state = self.progress['qualification']
        state['journeys'][0].update(state='passed', verified='old result', evidence='old.json')
        self.proof = {'build_id':'candidate-2', 'source_commit':'d'*40}
        self.progress['pending_join_packets']=[{'stream':'Keymap'}]
        with patch.object(q, 'git', return_value=''):
            self.assertTrue(q.refresh(self.progress, self.proof, self.provenance, Path('/unused')))
        active = self.progress['qualification']
        self.assertEqual(active['phase'], 'integrating')
        self.assertEqual(active['candidate_build_id'], 'candidate-2')
        self.assertEqual(active['journeys'], [{'id':'encoder-edit', 'state':'pending'}])
        self.assertEqual(active['history'][-1]['journeys'][0]['verified'], 'old result')

    def test_new_candidate_reuses_passed_journey_only_when_scoped_paths_are_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            evidence = root / 'receipt.md'
            evidence.write_text('paired evidence')
            source = root / 'web/src/keymap.rs'
            source.parent.mkdir(parents=True)
            source.write_text('source')
            scope = {'id':'keymap-functional', 'source_commit':'c'*40, 'streams':['Keymap'],
                     'journeys':[{'id':'encoder-edit', 'scope':'Edit and persist encoder',
                                  'spec':'issues/keymap.md', 'source_paths':['web/src/keymap.rs'],
                                  'source_paths_complete':True}]}
            state = self.progress['qualification']
            state.update(candidate_build_id='candidate-1', source_commit='c'*40,
                         phase='qualified', active_scope=scope,
                         journeys=[{'id':'encoder-edit', 'scope':'Edit and persist encoder',
                                    'spec':'issues/keymap.md', 'source_paths':['web/src/keymap.rs'],
                                    'source_paths_complete':True, 'state':'passed',
                                    'evidence':'receipt.md', 'verified':'paired pass'}],
                         review={'state':'completed', 'receipt':'review.md'})
            self.progress['qualification'] = state
            self.proof = {'build_id':'candidate-2', 'source_commit':'d'*40}
            self.provenance['commands'] = [{'argv':q.PAGE_CHECK, 'exit':0}]
            def git(root_arg, *args):
                if args[:3] == ('diff', '--no-renames', '--name-only'):
                    return ''
                return ''
            with patch.object(q, 'git', side_effect=git):
                self.assertTrue(q.refresh(self.progress, self.proof, self.provenance, root))
            active = self.progress['qualification']
            self.assertEqual(active['journeys'][0]['state'], 'passed')
            self.assertEqual(active['journeys'][0]['reuse'], 'unchanged_scoped_source')
            self.assertEqual(active['review']['state'], 'pending')
            self.assertFalse(q.refresh(self.progress, self.proof, self.provenance, root))

    def test_changed_scoped_path_resets_journey_and_changed_source_resets_review(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'receipt.md').write_text('paired evidence')
            source = root / 'web/src/keymap.rs'
            source.parent.mkdir(parents=True)
            source.write_text('source')
            scope = {'id':'keymap-functional', 'source_commit':'c'*40, 'streams':['Keymap'],
                     'journeys':[{'id':'encoder-edit', 'scope':'Edit and persist encoder',
                                  'spec':'issues/keymap.md', 'source_paths':['web/src/keymap.rs'],
                                  'source_paths_complete':True}]}
            state = self.progress['qualification']
            state.update(candidate_build_id='candidate-1', source_commit='c'*40,
                         phase='qualified', active_scope=scope,
                         journeys=[{'id':'encoder-edit', 'scope':'Edit and persist encoder',
                                    'spec':'issues/keymap.md', 'source_paths':['web/src/keymap.rs'],
                                    'source_paths_complete':True, 'state':'passed', 'evidence':'receipt.md'}],
                         review={'state':'completed', 'receipt':'review.md'})
            self.proof = {'build_id':'candidate-2', 'source_commit':'d'*40}
            self.provenance['commands'] = [{'argv':q.PAGE_CHECK, 'exit':0}]
            def git(root_arg, *args):
                if args[:3] == ('diff', '--no-renames', '--name-only'):
                    return 'web/src/keymap.rs\0'
                return ''
            with patch.object(q, 'git', side_effect=git):
                self.assertTrue(q.refresh(self.progress, self.proof, self.provenance, root))
            active = self.progress['qualification']
            self.assertEqual(active['journeys'][0], {
                'id':'encoder-edit', 'scope':'Edit and persist encoder', 'spec':'issues/keymap.md',
                'source_paths':['web/src/keymap.rs'], 'source_paths_complete':True, 'state':'pending'})
            self.assertEqual(active['review']['state'], 'pending')

    def test_rename_of_old_scoped_path_invalidates_reuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'receipt.md').write_text('paired evidence')
            source = root / 'web/src/new_keymap.rs'
            source.parent.mkdir(parents=True)
            source.write_text('renamed source')
            def git(root_arg, *args):
                if args[:3] == ('diff', '--no-renames', '--name-only'):
                    return 'web/src/keymap.rs\0web/src/new_keymap.rs\0'
                return ''
            with patch.object(q, 'git', side_effect=git):
                paths = q.reusable_journeys(
                    [{'id':'encoder-edit', 'scope':'Edit', 'source_paths':['web/src/keymap.rs'],
                      'source_paths_complete':True}],
                    [{'id':'encoder-edit', 'scope':'Edit', 'source_paths':['web/src/keymap.rs'],
                      'source_paths_complete':True, 'state':'passed', 'evidence':'receipt.md'}],
                    'c'*40, 'd'*40, root)
            self.assertEqual(paths[0]['state'], 'pending')

    def test_directory_footprint_is_not_reused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'receipt.md').write_text('paired evidence')
            (root/'web/src').mkdir(parents=True)
            def git(root_arg, *args):
                return ''
            with patch.object(q, 'git', side_effect=git):
                journeys = q.reusable_journeys(
                    [{'id':'encoder-edit', 'scope':'Edit', 'source_paths':['web/src'],
                      'source_paths_complete':True}],
                    [{'id':'encoder-edit', 'scope':'Edit', 'source_paths':['web/src'],
                      'source_paths_complete':True, 'state':'passed', 'evidence':'receipt.md'}],
                    'c'*40, 'd'*40, root)
            self.assertEqual(journeys[0]['state'], 'pending')

    def test_identical_source_candidate_still_requires_candidate_review(self):
        state = self.progress['qualification']
        state.update(candidate_build_id='candidate-1', source_commit='c'*40,
                     phase='qualified', active_scope={'id':'keymap-functional',
                         'source_commit':'c'*40, 'streams':['Keymap'], 'journeys':[{'id':'encoder-edit'}]},
                     journeys=[{'id':'encoder-edit', 'state':'partial'}],
                     review={'state':'completed', 'receipt':'review.md'})
        self.proof = {'build_id':'candidate-2', 'source_commit':'c'*40}
        self.provenance['commands'] = [{'argv':q.PAGE_CHECK, 'exit':0}]
        with patch.object(q, 'git', return_value=''):
            self.assertTrue(q.refresh(self.progress, self.proof, self.provenance, Path('/unused')))
        self.assertEqual(self.progress['qualification']['review']['state'], 'pending')

    def test_functional_scope_requires_nonempty_stream_names_and_journey_ids(self):
        self.progress['qualification']['next_scope']={
            'id':'invalid-scope', 'source_commit':'c'*40, 'streams':[''],
            'journeys':[{'scope':'No journey ID'}],
        }
        self.refresh()
        active = self.progress['qualification']
        self.assertEqual(active['phase'], 'integrating')
        self.assertTrue(any('stream scope' in reason for reason in active['unmet_start_conditions']))
        self.assertTrue(any('journey scope' in reason for reason in active['unmet_start_conditions']))

    def test_functional_scope_requires_an_id(self):
        self.progress['qualification']['next_scope']={
            'source_commit':'c'*40, 'streams':['Keymap'], 'journeys':[{'id':'encoder-edit'}],
        }
        self.refresh()
        active = self.progress['qualification']
        self.assertEqual(active['phase'], 'integrating')
        self.assertTrue(any('scope ID' in reason for reason in active['unmet_start_conditions']))

if __name__ == '__main__': unittest.main()
