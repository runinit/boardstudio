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

if __name__ == '__main__': unittest.main()
