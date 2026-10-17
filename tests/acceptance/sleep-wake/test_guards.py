import importlib.util
from pathlib import Path
import unittest
import tempfile
from unittest.mock import Mock
from types import SimpleNamespace

spec = importlib.util.spec_from_file_location('sleep_wake', Path(__file__).with_name('run.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class LifecycleFaultGuards(unittest.TestCase):
    def test_only_exact_synthetic_main_endpoint(self):
        self.assertEqual(module.endpoint_id('a' * 32), 'ep-' + 'a' * 32)
        for value in ['../production', 'a' * 31, 'A' * 32, 'a' * 32 + '-rc-any']:
            with self.assertRaises(AssertionError):
                module.endpoint_id(value)

    def test_only_actual_api_binary_and_bound_parent(self):
        module.guard_api(['/usr/local/bin/briven_control_api'], 1)
        for argv, parent in [(['sleep', '10'], 1), (['briven_control_api', 'other'], 1),
                             (['/usr/local/bin/briven_control_api'], 999)]:
            with self.assertRaises(AssertionError):
                module.guard_api(argv, parent)

    def test_barriers_and_signals_are_closed_sets(self):
        self.assertEqual(module.BARRIERS, {'before-stop', 'after-stop', 'before-start', 'after-start'})
        self.assertEqual(module.SIGNALS, {'STOP', 'CONT', 'KILL'})
        self.assertNotIn('TERM', module.SIGNALS)

    def test_failed_probes_do_not_prove_api_handoff(self):
        self.assertTrue(module.api_exit_observed(SimpleNamespace(returncode=0, stdout='absent\n')))
        self.assertTrue(module.api_exit_observed(SimpleNamespace(returncode=0, stdout='23 (briven_control) Z 1')))
        self.assertFalse(module.api_exit_observed(SimpleNamespace(returncode=0, stdout='23 (briven_control) S 1')))
        with self.assertRaises(AssertionError):
            module.api_exit_observed(SimpleNamespace(returncode=1, stdout=''))
        module.guard_connection_refused(SimpleNamespace(returncode=7, stderr='connect to 127.0.0.1 port 8787 failed: Connection refused'))
        for code, detail in [(22, 'HTTP 503'), (28, 'Connection timed out'), (1, 'Docker unavailable'), (7, 'Network unreachable')]:
            with self.assertRaises(AssertionError):
                module.guard_connection_refused(SimpleNamespace(returncode=code, stderr=detail))

    def test_process_disappearing_between_reads_requires_positive_exit(self):
        for phase, exited in [(phase, exited) for phase in ('identity', 'second-stat') for exited in (True, False, None)]:
            proof = module.Acceptance.__new__(module.Acceptance)
            proof.endpoint = Mock(return_value='ep-' + 'a' * 32)
            proof.row = Mock(return_value={'Id': 'owned-ci-container'})
            proof.cmd = Mock(return_value=SimpleNamespace(returncode=0, stdout='42'))
            proof.identity = Mock(side_effect=RuntimeError('read raced exit')) if phase == 'identity' else Mock(return_value=(['postgres'], 1))
            proof.engine = Mock(side_effect=RuntimeError('second stat raced exit'))
            proof.process_exited = Mock(side_effect=AssertionError('failed exit probe')) if exited is None else Mock(return_value=exited)
            proof.report = Mock()
            if exited:
                self.assertEqual(proof.processes(), {'postgres': None, 'controller': None})
            elif exited is False:
                with self.assertRaises(RuntimeError): proof.processes()
            else:
                with self.assertRaises(AssertionError): proof.processes()

    def test_live_wrong_identity_is_not_normalized_as_exit(self):
        proof = module.Acceptance.__new__(module.Acceptance)
        proof.endpoint = Mock(return_value='ep-' + 'a' * 32)
        proof.row = Mock(return_value={'Id': 'owned-ci-container'})
        proof.cmd = Mock(return_value=SimpleNamespace(returncode=0, stdout='42'))
        proof.identity = Mock(return_value=(['unrelated-process'], 1))
        proof.engine = Mock(return_value='42 (unrelated) S 1')
        proof.process_exited = Mock(return_value=True)
        with self.assertRaises(AssertionError): proof.processes()
        proof.process_exited.assert_not_called()

    def test_pid_file_and_readability_errors_are_not_absence(self):
        proof = module.Acceptance.__new__(module.Acceptance)
        proof.endpoint = Mock(return_value='ep-' + 'a' * 32)
        proof.row = Mock(return_value={'Id': 'owned-ci-container'})
        proof.process_exited = Mock(return_value=False)
        for code, contents in [(125, ''), (45, ''), (0, 'broken'), (0, '')]:
            proof.cmd = Mock(return_value=SimpleNamespace(returncode=code, stdout=contents))
            with self.assertRaises(AssertionError): proof.processes()
        proof.cmd = Mock(side_effect=[SimpleNamespace(returncode=0, stdout='42'), SimpleNamespace(returncode=125, stdout='')])
        with self.assertRaises(AssertionError): proof.processes()

    def test_reconciliation_requires_proven_recent_exit_before_original_deadline(self):
        record = {'phase': 'stopping', 'stopRequested': True, 'failure': None, 'phaseSince': 10}
        self.assertTrue(module.reconciliation_eligible(record, True, 15000, 20000))
        self.assertFalse(module.reconciliation_eligible(record, False, 15000, 20000))
        self.assertFalse(module.reconciliation_eligible(record, True, 21000, 20000))
        self.assertFalse(module.reconciliation_eligible(record, True, 46000, 50000))
        for key, value in [('phase', 'running'), ('phase', 'failed'), ('stopRequested', False), ('failure', 'activity_unavailable')]:
            self.assertFalse(module.reconciliation_eligible({**record, key: value}, True, 15000, 20000))

    def test_private_files_are_removed_even_when_inventory_cleanup_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            proof = module.Acceptance.__new__(module.Acceptance)
            proof.root = Path(temporary); proof.started = False; proof.baseline = []
            proof.draining_client = proof.capacity_server = None
            proof.summary = {}; proof.report = Mock()
            proof.cmd = Mock(side_effect=RuntimeError('unavailable'))
            secret = proof.root / 'client-private.json'; secret.write_text('owned disposable credential')
            with self.assertRaises(AssertionError): proof.cleanup()
            self.assertFalse(secret.exists())
            self.assertFalse(proof.summary['cleanupPassed'])

    def test_missing_engine_does_not_bypass_owned_namespace_teardown(self):
        with tempfile.TemporaryDirectory() as temporary:
            proof = module.Acceptance.__new__(module.Acceptance)
            proof.root = Path(temporary); proof.started = True; proof.baseline = []
            proof.project = 'briven-durability-123-1'; proof.compose = ['docker', 'compose', '-p', proof.project]
            proof.draining_client = proof.capacity_server = None
            proof.summary = {}; proof.report = Mock(); calls = []
            def command(args, **kwargs):
                calls.append(args)
                if args[:2] == ['docker', 'inspect']:
                    return type('Missing', (), {'returncode': 1})()
                return ''
            proof.cmd = command
            proof.cleanup()
            self.assertTrue(any('down' in call and '--volumes' in call for call in calls))
            self.assertTrue(proof.summary['cleanupPassed'])


if __name__ == '__main__':
    unittest.main()
