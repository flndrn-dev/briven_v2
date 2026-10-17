import importlib.util
from pathlib import Path
import unittest

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


if __name__ == '__main__':
    unittest.main()
