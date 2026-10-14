"""Protect durable controller data when its PID namespace restarts."""
from pathlib import Path
import subprocess
import tempfile
import unittest


class ControllerRestartTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='briven-controller-restart-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.data = self.root / 'controller'
        self.proc = self.root / 'proc'
        self.sockets = self.root / 'sockets'
        for path in (self.data, self.proc, self.sockets):
            path.mkdir()
        (self.proc / '1').mkdir()
        (self.proc / '1' / 'comm').write_text('sh\n')
        self.binary = '/usr/local/v16/bin/postgres'
        self.pidfile = self.data / 'postmaster.pid'
        self.socket = self.sockets / '.s.PGSQL.1235'
        self.lock = self.sockets / '.s.PGSQL.1235.lock'
        self.pidfile.write_text(f'42\n{self.data}\n100\n1235\n{self.sockets}\nlocalhost\n0\nready\n')
        self.lock.write_text(f'42\n{self.data}\n100\n1235\n{self.sockets}\nlocalhost\n')
        self.socket.write_text('stale socket fixture')
        self.durable = self.data / 'durable-data'
        self.durable.write_text('acknowledged controller state')

    def process(self, name, cwd=None, executable=None):
        path = self.proc / '42'
        path.mkdir()
        (path / 'comm').write_text(name + '\n')
        if cwd is not None:
            (path / 'cwd').symlink_to(cwd)
        if executable is not None:
            (path / 'exe').symlink_to(executable)

    def run_guard(self, scope='controller'):
        source = Path(__file__).with_name('start-private.sh').read_text()
        begin = source.index('# BEGIN CONTROLLER RESTART GUARD')
        end = source.index('# END CONTROLLER RESTART GUARD', begin)
        result = subprocess.run(['sh', '-c', source[begin:end] + '\nprepare_postmaster_restart "$1" "$2" "$3" "$4" "$5"',
                                 'controller-restart-test', str(self.data), str(self.proc), self.binary, str(self.sockets), scope],
                                capture_output=True, text=True)
        self.assertEqual(self.durable.read_text(), 'acknowledged controller state')
        return result.returncode

    def assert_retained(self):
        for path in (self.pidfile, self.socket, self.lock):
            self.assertTrue(path.exists())

    def assert_cleared(self):
        for path in (self.pidfile, self.socket, self.lock):
            self.assertFalse(path.exists())

    def test_recycled_safekeeper_pid_does_not_block_postgres_recovery(self):
        self.process('safekeeper', self.root, '/usr/local/bin/safekeeper')
        self.assertEqual(self.run_guard(), 0)
        self.assert_cleared()

    def test_absent_old_process_clears_only_its_runtime_locks(self):
        self.assertEqual(self.run_guard(), 0)
        self.assert_cleared()

    def test_live_controller_postmaster_is_preserved(self):
        self.process('postgres', self.data, self.binary)
        self.assertEqual(self.run_guard(), 0)
        self.assert_retained()

    def test_foreign_postgres_process_is_not_signalled_or_modified(self):
        self.process('postgres', self.root, self.binary)
        self.assertEqual(self.run_guard(), 0)
        self.assert_cleared()
        self.assertEqual((self.proc / '42' / 'comm').read_text(), 'postgres\n')

    def test_unknown_postgres_identity_refuses_cleanup(self):
        self.process('postgres')
        self.assertNotEqual(self.run_guard(), 0)
        self.assert_retained()

    def test_malformed_pid_refuses_cleanup(self):
        self.pidfile.write_text('not-a-pid\n')
        self.assertNotEqual(self.run_guard(), 0)
        self.assert_retained()

    def test_different_data_directory_refuses_cleanup(self):
        self.pidfile.write_text(f'42\n{self.root}\n100\n1235\n{self.sockets}\n')
        self.assertNotEqual(self.run_guard(), 0)
        self.assert_retained()

    def test_foreign_socket_lock_refuses_all_cleanup(self):
        self.lock.write_text(f'99\n{self.root}\n')
        self.assertNotEqual(self.run_guard(), 0)
        self.assert_retained()

    def test_no_pid_file_does_not_remove_unowned_socket(self):
        self.pidfile.unlink()
        self.assertEqual(self.run_guard(), 0)
        self.assertTrue(self.socket.exists())
        self.assertTrue(self.lock.exists())

    def test_unavailable_process_namespace_refuses_cleanup(self):
        (self.proc / '1' / 'comm').unlink()
        self.assertNotEqual(self.run_guard(), 0)
        self.assert_retained()

    def compute_socket(self):
        self.pidfile.write_text(self.pidfile.read_text().replace('\n1235\n','\n55440\n'))
        self.lock.write_text(self.lock.read_text().replace('\n1235\n','\n55440\n'))
        self.socket = self.socket.rename(self.sockets / '.s.PGSQL.55440')
        self.lock = self.lock.rename(self.sockets / '.s.PGSQL.55440.lock')

    def test_compute_socket_pid_reuse_preserves_the_crash_review_marker(self):
        self.compute_socket()
        self.process('safekeeper',self.root,'/usr/local/bin/safekeeper')
        self.assertEqual(self.run_guard('compute'),0)
        self.assertTrue(self.pidfile.exists())
        self.assertFalse(self.lock.exists())
        self.assertFalse(self.socket.exists())

    def test_running_compute_socket_and_marker_are_preserved(self):
        self.compute_socket()
        self.process('postgres',self.data,self.binary)
        self.assertEqual(self.run_guard('compute'),0)
        self.assert_retained()

    def test_invalid_compute_port_refuses_cleanup(self):
        self.pidfile.write_text(self.pidfile.read_text().replace('\n1235\n','\n70000\n'))
        self.assertNotEqual(self.run_guard('compute'),0)
        self.assert_retained()

    def test_unknown_scope_refuses_cleanup(self):
        self.assertNotEqual(self.run_guard('unknown'),0)
        self.assert_retained()


if __name__ == '__main__':
    unittest.main()
