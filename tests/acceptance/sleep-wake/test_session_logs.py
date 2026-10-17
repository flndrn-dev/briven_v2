import importlib.util
import json
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('session_logs', Path(__file__).with_name('session_logs.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class SessionEvidence(unittest.TestCase):
    def test_only_classified_metadata_survives_raw_log_projection(self):
        role = 'briven_' + 'a'*32
        raw = (
            'PG:2026-10-17 20:00:00.100 UTC [42] BRIVEN_ACCEPTANCE_SESSION app=compute_ctl:compute_monitor| LOG:  connection authorized: user=cloud_admin database=postgres application_name=compute_ctl:compute_monitor\n'
            'PG:2026-10-17 20:00:00.125 UTC [42] BRIVEN_ACCEPTANCE_SESSION app=compute_ctl:compute_monitor| LOG:  disconnection: session time: 0:00:00.025 user=cloud_admin database=postgres host=127.0.0.1\n'
            'PG:2026-10-17 20:00:00.130 UTC [43] BRIVEN_ACCEPTANCE_SESSION app=secret-password| LOG:  statement: SELECT private_token\n'
            'PG:2026-10-17 20:00:01.100 UTC [44] BRIVEN_ACCEPTANCE_SESSION app=briven_lifecycle_inspection| LOG:  disconnection: session time: 0:00:00.100 user='+role+' database=postgres host=127.0.0.1\n'
        )
        summary = module.summarize(raw, role, 1792267200)
        self.assertEqual(summary['completed']['monitor'], 1)
        self.assertEqual(summary['completed']['customer'], 1)
        self.assertEqual(summary['maxDurationMs']['monitor'], 25)
        self.assertEqual(summary['nearDemandTotal'], 2)
        text = json.dumps(summary)
        for secret in ('secret-password', 'private_token', 'SELECT', role, 'cloud_admin'):
            self.assertNotIn(secret, text)

    def test_injected_pause_is_separated_from_normal_lifetime(self):
        raw = 'PG:2026-10-17 20:00:10.000 UTC [8] BRIVEN_ACCEPTANCE_SESSION app=compute_ctl:compute_monitor| LOG:  disconnection: session time: 0:00:10.000 user=cloud_admin database=postgres host=127.0.0.1\u200b'
        summary = module.summarize(raw, 'briven_'+'a'*32, 1792267205, (1792267201,1792267209))
        self.assertEqual(summary['maxDurationMs']['monitor'], 10000)
        self.assertEqual(summary['maxDurationOutsidePauseMs']['monitor'], 0)
        self.assertTrue(summary['nearDemand'][0]['overlapsInjectedPause'])

    def test_malformed_and_unrelated_log_text_are_not_session_events(self):
        summary = module.summarize('private credentials\nnot a PostgreSQL session event', 'briven_'+'a'*32, 0)
        self.assertEqual(summary['eventsParsed'], 0)
        self.assertEqual(sum(summary['completed'].values()), 0)

    def test_unfinished_monitor_is_explicitly_unpaired_with_observed_age(self):
        raw = 'PG:2026-10-17 20:00:00.000 UTC [42] BRIVEN_ACCEPTANCE_SESSION app=compute_ctl:compute_monitor| LOG:  connection authorized: user=cloud_admin database=postgres\n'
        summary = module.summarize(raw, 'briven_'+'a'*32, 1792267210, observed_at=1792267220, bytes_read=2097152)
        self.assertEqual(summary['unpairedByClass']['monitor'], 1)
        self.assertEqual(summary['unpaired'][0]['observedAgeMs'], 20000)
        self.assertEqual(summary['unpaired'][0]['pairStatus'], 'unpaired-log-events')
        self.assertEqual(summary['unpairedNearDemandTotal'], 1)
        self.assertTrue(summary['coverage']['tailMayBeTruncated'])
        self.assertFalse(summary['coverage']['demandIntervalWithinTaggedSpan'])
        self.assertNotIn('cloud_admin', json.dumps(summary))


if __name__ == '__main__':
    unittest.main()
