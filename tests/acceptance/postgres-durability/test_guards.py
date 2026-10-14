import copy
import unittest
from run import guard_container, ENGINE_IMAGE, startup_signals, redacted_cli_error


class FaultGuardTests(unittest.TestCase):
    def test_controlled_cli_diagnostic_redacts_credentials_and_keys(self):
        password = 'a' * 64
        jwt = 'eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJmaXh0dXJlIn0.signature123456789'
        raw = f'command failed: compute startup failed: invalid WAL record\npostgres://user:{password}@host/db\n{jwt}\npassword: short-password\n-----BEGIN PRIVATE KEY-----\nkey-material\n-----END PRIVATE KEY-----'
        result = redacted_cli_error(raw)
        self.assertIn('invalid WAL record', result)
        for value in (password, jwt, 'short-password', 'key-material', 'postgres://'):
            self.assertNotIn(value, result)

    def test_diagnostics_never_return_raw_log_secrets(self):
        secret = 'fixture-password-should-never-appear'
        raw = f'pg_ctl: server is running (PID: 42)\nConnection refused postgres://user:{secret}@host/db'
        signals = startup_signals(raw)
        self.assertEqual(signals, ['postgres-running-reported', 'connection-refused'])
        self.assertNotIn(secret, str(signals))

    def setUp(self):
        self.project = 'briven-durability-123456-1'
        self.row = {
            'Name': '/' + self.project + '-engine-1',
            'Config': {'Image': ENGINE_IMAGE, 'Labels': {
                'com.docker.compose.project': self.project,
                'com.docker.compose.service': 'engine',
                'briven.acceptance.purpose': 'postgres-durability'}},
            'HostConfig': {'PortBindings': {}, 'Privileged': False,
                           'PidMode': '', 'Binds': None},
            'Mounts': [{'Type': 'volume', 'Name': self.project + '_repo',
                        'Source': '/var/lib/docker/volumes/test/_data',
                        'Destination': '/var/lib/briven', 'RW': True}],
        }

    def test_valid_owned_fixture_is_accepted(self):
        guard_container(self.row, self.project, 'engine')

    def test_production_container_name_is_rejected(self):
        self.row['Name'] = '/briven-clean-engine-20261011-dsjaqo-engine-1'
        with self.assertRaises(AssertionError):
            guard_container(self.row, self.project, 'engine')

    def test_another_run_or_service_is_rejected(self):
        for key, value in [('com.docker.compose.project', 'other'),
                           ('com.docker.compose.service', 'catalog'),
                           ('briven.acceptance.purpose', 'production')]:
            row = copy.deepcopy(self.row)
            row['Config']['Labels'][key] = value
            with self.assertRaises(AssertionError):
                guard_container(row, self.project, 'engine')

    def test_customer_volume_is_rejected(self):
        self.row['Mounts'][0]['Name'] = 'briven-clean-engine-20261011-dsjaqo_data'
        with self.assertRaises(AssertionError):
            guard_container(self.row, self.project, 'engine')

    def test_image_declared_anonymous_volume_is_rejected(self):
        self.row['Mounts'].append({'Type': 'volume', 'Name': 'f' * 64,
                                  'Source': '/var/lib/docker/volumes/anonymous/_data',
                                  'Destination': '/data', 'RW': True})
        with self.assertRaises(AssertionError):
            guard_container(self.row, self.project, 'engine')

    def test_host_control_mount_and_public_port_are_rejected(self):
        row = copy.deepcopy(self.row)
        row['Mounts'].append({'Type': 'bind', 'Source': '/run/briven-compute-control',
                             'Destination': '/run/briven-compute-control', 'RW': False})
        with self.assertRaises(AssertionError):
            guard_container(row, self.project, 'engine')
        self.row['HostConfig']['PortBindings'] = {'5432/tcp': [{'HostPort': '5432'}]}
        with self.assertRaises(AssertionError):
            guard_container(self.row, self.project, 'engine')

    def test_changed_engine_image_is_rejected(self):
        self.row['Config']['Image'] = 'unverified:latest'
        with self.assertRaises(AssertionError):
            guard_container(self.row, self.project, 'engine')

    def test_compose_resolved_immutable_config_id_is_verified(self):
        from run import IMAGES
        config_id = IMAGES['engine'][1]
        self.row['Config']['Image'] = config_id
        self.row['Image'] = config_id
        guard_container(self.row, self.project, 'engine')
        self.row['Image'] = 'sha256:' + '0' * 64
        with self.assertRaises(AssertionError):
            guard_container(self.row, self.project, 'engine')


if __name__ == '__main__':
    unittest.main()
