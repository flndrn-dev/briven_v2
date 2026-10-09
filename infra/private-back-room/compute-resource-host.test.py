"""Protocol/admission proofs only; this does not claim Linux kernel acceptance."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("resource_host", Path(__file__).with_name("compute-resource-host.py"))
host = importlib.util.module_from_spec(spec)
spec.loader.exec_module(host)


class ResourceProtocol(unittest.TestCase):
    def test_no_caller_pid_command_limits_or_foreign_binding(self):
        valid = {"version": 1, "action": "prepare", "endpoint": "ep-" + "a" * 32, "project": "a" * 32}
        host.validate_request(valid)
        bad = [dict(valid, pid=1), dict(valid, command="/bin/sh"), dict(valid, memory=999),
               dict(valid, project="b" * 32), dict(valid, endpoint="ep-../other"),
               dict(valid, version=True), dict(valid, action="stop"),
               dict(valid, endpoint="ep-" + "a" * 32 + "-"),
               dict(valid, endpoint="ep-" + "a" * 32 + "/other")]
        for request in bad:
            with self.assertRaises(ValueError):
                host.validate_request(request)

    def test_exact_global_and_project_capacity(self):
        a, b = "a" * 32, "b" * 32
        active = ["ep-" + a, "ep-" + a + "-branch"]
        with self.assertRaises(ValueError):
            host.admission(a, active)
        host.admission(b, active)
        with self.assertRaises(ValueError):
            host.admission(b, active + ["ep-" + b, "ep-" + b + "-branch"])

    def test_duplicate_json_authority_fields_are_refused(self):
        with self.assertRaises(ValueError):
            host.json.loads('{"version":1,"version":2}', object_pairs_hook=host.unique_fields)

    def test_scope_request_has_only_fixed_properties_and_authenticated_pid(self):
        endpoint, pid = "ep-" + "a" * 32, 4242
        with tempfile.TemporaryDirectory() as directory:
            group = Path(directory) / ("briven-compute-" + endpoint + ".scope")
            group.mkdir()
            for key, value in host.LIMITS.items():
                (group / key).write_text(value)
            (group / "cgroup.events").write_text("populated 0\n")
            (group / "cgroup.procs").write_text(str(pid) + "\n")
            with patch.object(host, "CGROUP", Path(directory)), \
                 patch.object(host, "start_identity", return_value="123"), \
                 patch.object(host.os, "pidfd_open", return_value=99, create=True), \
                 patch.object(host.os, "close"), \
                 patch.object(host.select, "select", return_value=([], [], [])), \
                 patch.object(host.subprocess, "run"), \
                 patch.object(host, "command", return_value="job") as call:
                host.prepare(endpoint, pid, "123")
                args = call.call_args.args[0]
            self.assertEqual(args[7:10], ["briven-compute-" + endpoint + ".scope", "fail", "11"])
            properties = args[10:-1]
            decoded = {}
            while properties:
                name, kind = properties[:2]
                length = 4 if kind == "au" else 3
                decoded[name] = properties[2:length]
                properties = properties[length:]
            self.assertEqual(len(decoded), 11)
            self.assertEqual(decoded["PIDs"], ["1", str(pid)])
            self.assertEqual(decoded["Slice"], [host.SLICE])
            self.assertEqual(decoded["MemoryMax"], ["536870912"])
            self.assertEqual(decoded["CPUQuotaPerSecUSec"], ["500000"])
            self.assertEqual(decoded["OOMPolicy"], ["kill"])

    def test_unknown_kernel_values_are_not_treated_as_empty_or_unlimited(self):
        with tempfile.TemporaryDirectory() as directory:
            group = Path(directory)
            (group / "cgroup.events").write_text("populated unknown\n")
            with self.assertRaises(ValueError):
                host.populated(group)
            for key, value in host.LIMITS.items():
                (group / key).write_text(value)
            (group / "memory.max").write_text("max")
            with self.assertRaises(ValueError):
                host.limits(group, host.LIMITS)


if __name__ == "__main__":
    unittest.main()
