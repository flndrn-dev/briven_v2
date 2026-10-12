import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('metrics', Path(__file__).with_name('compute-resource-metrics.py'))
metrics = importlib.util.module_from_spec(spec)
spec.loader.exec_module(metrics)


class MetricsTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name).resolve()
        self.group = self.base / 'brivencompute.slice'
        self.group.mkdir()
        self.output = self.base / 'compute.prom'
        for name, value in {**metrics.BOUNDS, 'cpu.max': '300000 100000',
                            'cpu.stat': 'usage_usec 12500000\nuser_usec 8000000\nsystem_usec 4500000\n',
                            'memory.current': 1048576, 'pids.current': 29}.items():
            (self.group / name).write_text(str(value))

    def test_aggregate_parent_values_and_real_collection_time(self):
        # Deliberately different child values: parent accounting already includes descendants.
        child = self.group / 'child.scope'
        child.mkdir()
        (child / 'memory.current').write_text('999999')
        result = metrics.collect(self.group, 12345)
        self.assertEqual(result['memory_current_bytes'], 1048576)
        self.assertEqual(result['cpu_usage_seconds_total'], 12.5)
        self.assertEqual(result['tasks_current'], 29)
        self.assertEqual(result['sampled_timestamp_seconds'], 12345)
        self.assertTrue(metrics.publish(self.output, self.group, 12345))
        self.assertIn('sampled_timestamp_seconds{group="brivencompute.slice"} 12345', self.output.read_text())

    def test_changed_unlimited_or_missing_bounds_replace_previous_success(self):
        for name, value in [('cpu.max', 'max 100000'), ('memory.max', 'max'),
                            ('pids.max', '2048'), ('memory.swap.max', '1')]:
            original = (self.group / name).read_text()
            self.assertTrue(metrics.publish(self.output, self.group, 12345))
            (self.group / name).write_text(value)
            self.assertFalse(metrics.publish(self.output, self.group, 12346))
            self.assertEqual(self.output.read_text(), 'briven_compute_collection_success{group="brivencompute.slice"} 0\n')
            (self.group / name).write_text(original)
        (self.group / 'pids.current').unlink()
        self.assertFalse(metrics.publish(self.output, self.group))

    def test_malformed_oversized_and_impossible_accounting_is_unknown(self):
        for name, value in [('cpu.stat', 'usage_usec 1\nusage_usec 2'),
                            ('cpu.stat', 'user_usec 1'), ('cpu.stat', 'bad'),
                            ('cpu.stat', 'x' * 4097), ('memory.current', '-1'),
                            ('memory.current', '8589934593'), ('pids.current', '1025')]:
            original = (self.group / name).read_text()
            (self.group / name).write_text(value)
            self.assertFalse(metrics.publish(self.output, self.group))
            (self.group / name).write_text(original)

    def test_symlink_input_and_output_are_refused(self):
        (self.group / 'memory.current').unlink()
        (self.group / 'memory.current').symlink_to(self.group / 'memory.max')
        self.assertFalse(metrics.publish(self.output, self.group))
        self.output.unlink()
        self.output.symlink_to(self.group / 'pids.max')
        with self.assertRaises(ValueError):
            metrics.publish(self.output, self.group)
        self.assertEqual((self.group / 'pids.max').read_text(), '1024')


if __name__ == '__main__':
    unittest.main()
