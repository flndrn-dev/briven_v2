"""Real isolated engine acceptance. Fault injection is limited to its CI namespace."""
import datetime
import json
import os
from pathlib import Path
import re
import secrets
import selectors
import shutil
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
ENGINE_SHA = '743cab2d44712ad3d81f75563c98bf6c06f5f427'
ENGINE_IMAGE = 'ghcr.io/flndrn-dev/briven-engine@sha256:4bf8842f589e7c774d1742c80f9789964152ac7b86295c08ee7f804305c4e9a8'
IMAGES = {
    'engine': (ENGINE_IMAGE, 'sha256:b045b0c105ce8598f83205d094d7803bf7b624fb66b3bb2a751e62fae2f5eaac'),
    'catalog': ('postgres@sha256:65b16a8b326e0cfbdf33fa7e783f2a0cb352a61448616ccccfd616ef42aa0f65', 'sha256:275447c94b11b151decd1f29877965301d5a77032037c46de93d990f739f00a9'),
    'minio': ('bitnamilegacy/minio@sha256:451fe6858cb770cc9d0e77ba811ce287420f781c7c1b806a386f6896471a349c', 'sha256:ed85a6fd1548f908ab978af49ecc724039ea945c6aef8fb7083a29d67838e403'),
    'minio-init': ('bitnamilegacy/minio-client@sha256:00dcc4e58ada0df45bb7d9ee435af98295f96c27c3c68292ce78ec700a87b511', 'sha256:3a3daed2aa1f8b4ead16fc89e82d52697111b306bcb614e09121c1cd57d55d9f'),
}
SERVICES = {'engine', 'catalog', 'minio', 'minio-init', 'certificates'}
STARTUP_SIGNALS = {
    'postgres-running-reported': r'pg_ctl: server is running',
    'postgres-start-attempted': r'Starting storage controller database',
    'postgres-ready': r'database system is ready to accept connections',
    'postgres-recovery-started': r'database system was interrupted|automatic recovery in progress',
    'postgres-pid-lock-conflict': r'lock file .* already exists|another server might be running',
    'postgres-invalid-checkpoint': r'invalid checkpoint|could not locate a valid checkpoint',
    'connection-refused': r'Connection refused|connection refused',
    'startup-timeout': r'did not start\+pass status checks|Timed out waiting',
    'startup-failed': r'startup failed because|error starting process',
    'postgres-fatal-or-panic': r'FATAL:|PANIC:',
}


def startup_signals(raw):
    # Emit fixed categories only; never return arbitrary log text or credentials.
    return [name for name, pattern in STARTUP_SIGNALS.items() if re.search(pattern, raw)]

BINDS = {
    str(REPO / 'infra/private-back-room/start-private.sh'): '/start-private.sh',
    str(REPO / 'infra/private-back-room/local-env.toml'): '/local-env.toml',
    str(REPO / 'infra/private-back-room/create-proxy-certificates.sh'): '/create-proxy-certificates.sh',
}


def guard_container(row, project, service):
    assert re.fullmatch(r'briven-durability-[0-9]+-[0-9]+', project), 'invalid fixture namespace'
    assert service in SERVICES, 'unexpected fixture service'
    assert row['Name'] == '/' + project + '-' + service + '-1', 'container name does not match owned fixture'
    labels = row['Config']['Labels']
    assert labels['com.docker.compose.project'] == project, 'container project does not match owned fixture'
    assert labels['com.docker.compose.service'] == service, 'container service does not match owned fixture'
    assert labels['briven.acceptance.purpose'] == 'postgres-durability', 'fixture purpose label missing'
    expected, config_id = IMAGES['engine' if service == 'certificates' else service]
    # Compose can resolve a pinned manifest into its immutable image configuration ID.
    assert row['Config']['Image'] in (expected, config_id), 'container immutable image reference differs'
    if 'Image' in row:
        assert row['Image'] == config_id, 'container actual image configuration identity differs'
    host = row['HostConfig']
    assert not host['PortBindings'] and not host['Privileged'], 'public ports or privileged container refused'
    assert host.get('PidMode', '') == '' and not host.get('CapAdd'), 'host PID access or extra capabilities refused'
    for mount in row['Mounts']:
        if mount['Type'] == 'volume':
            assert mount['Name'].startswith(project + '_'), 'foreign volume refused'
        else:
            assert mount['Type'] == 'bind' and not mount['RW'], 'unexpected writable host mount refused'
            assert BINDS.get(mount['Source']) == mount['Destination'], 'host mount is outside fixture source whitelist'
    if 'NetworkSettings' in row:
        assert set(row['NetworkSettings']['Networks']) == {project + '_fixture'}, 'network outside owned fixture refused'


class Acceptance:
    def __init__(self):
        assert os.environ.get('GITHUB_ACTIONS') == 'true' and sys.platform == 'linux'
        run_id, attempt = os.environ['GITHUB_RUN_ID'], os.environ['GITHUB_RUN_ATTEMPT']
        assert run_id.isdigit() and attempt.isdigit()
        self.project = f'briven-durability-{run_id}-{attempt}'
        temp = Path(os.environ['RUNNER_TEMP']).resolve()
        self.root = temp / self.project
        assert not self.root.exists(), 'Fresh CI fixture required; do not overwrite a journal'
        self.root.mkdir(mode=0o700)
        self.evidence = self.root / 'evidence.jsonl'
        self.summary = {'engineSource': ENGINE_SHA, 'engineImage': ENGINE_IMAGE,
                        'harnessSource': os.environ['GITHUB_SHA'], 'run': run_id,
                        'attempt': attempt, 'passed': False, 'cleanupPassed': False}
        self.started = False
        self.phase = 'preflight'
        self.paused_sks = set()
        self.minio_paused = False
        self.probe = None
        self.recovery_since = None
        self.env = dict(os.environ)
        self.env['BRIVEN_FIXTURE_ROOT'] = str(self.root)
        self.envfile = self.root / 'fixture-private.env'
        values = {'FIXTURE_' + key: secrets.token_hex(32) for key in
                  ['IDENTITY_SECRET', 'PROXY_SECRET', 'CATALOG_PASSWORD', 'OBJECT_PASSWORD']}
        self.identity_secret = values['FIXTURE_IDENTITY_SECRET']
        self.envfile.write_text(''.join(key + '=' + value + '\n' for key, value in values.items()))
        self.envfile.chmod(0o600)
        self.compose = ['docker', 'compose', '-p', self.project, '--env-file', str(self.envfile),
                        '-f', str(HERE / 'compose.yml')]
        self.baseline = self.cmd(['docker', 'ps', '-q']).split()

    def report(self, check, **values):
        record = {'check': check, 'time': datetime.datetime.now(datetime.timezone.utc).isoformat(), **values}
        line = json.dumps(record)
        print(line, flush=True)
        with self.evidence.open('a') as output:
            output.write(line + '\n')

    def cmd(self, args, timeout=60, check=True):
        result = subprocess.run(args, capture_output=True, text=True, timeout=timeout, env=self.env)
        if check and result.returncode != 0:
            raise RuntimeError('Fixed acceptance subprocess failed; raw output suppressed')
        return result.stdout if check else result

    def row(self, service):
        name = self.project + '-' + service + '-1'
        row = json.loads(self.cmd(['docker', 'inspect', name]))[0]
        guard_container(row, self.project, service)
        return row

    def engine(self, *args, timeout=60):
        row = self.row('engine')
        return self.cmd(['docker', 'exec', row['Id'], *args], timeout=timeout)

    def wait_healthy(self):
        deadline = time.monotonic() + 240
        while time.monotonic() < deadline:
            row = self.row('engine')
            if row['State'].get('Health', {}).get('Status') == 'healthy':
                version = self.engine('briven_local', '--version')
                assert ENGINE_SHA in version
                self.report('matching compiled engine is healthy', result='pass', sourceSha=ENGINE_SHA)
                self.client_config()
                return
            if not row['State']['Running']:
                raise RuntimeError('Owned fixture engine exited during startup')
            time.sleep(2)
        raise TimeoutError('Owned engine health deadline exceeded')

    def client_config(self):
        row = self.row('engine')
        address = row['NetworkSettings']['Networks'][self.project + '_fixture']['IPAddress']
        assert re.fullmatch(r'[0-9.]+', address)
        hosts = Path('/etc/hosts').read_text()
        if 'briven-customer-db' not in hosts:
            result = subprocess.run(['sudo', 'tee', '-a', '/etc/hosts'], input=address + ' briven-customer-db\n', text=True, capture_output=True)
            assert result.returncode == 0
        else:
            assert address + ' briven-customer-db' in hosts
        ca = self.engine('cat', '/etc/briven/proxy-ca/ca.crt')
        assert 'BEGIN CERTIFICATE' in ca and 'PRIVATE KEY' not in ca
        config = {'project': self.project, 'origin': 'http://' + address + ':8787',
                  'identitySecret': self.identity_secret, 'ca': ca}
        path = self.root / 'client-private.json'
        path.write_text(json.dumps(config)); path.chmod(0o600)

    def sk_signal(self, sk_id, action):
        assert sk_id in (1, 2) and action in ('STOP', 'CONT')
        raw = self.engine('cat', f'/var/lib/briven/safekeepers/sk{sk_id}/safekeeper.pid').strip()
        assert raw.isdigit() and int(raw) > 1
        assert self.engine('readlink', f'/proc/{raw}/exe').strip() == '/usr/local/bin/safekeeper'
        # The pinned slim image has a POSIX shell; it need not contain procps'
        # standalone kill executable. Keep the validated signal/PID positional.
        self.engine('sh', '-c', 'kill "-$1" "$2"', 'briven-fixture-signal', action, raw)
        (self.paused_sks.add if action == 'STOP' else self.paused_sks.discard)(sk_id)
        self.report('owned safekeeper fault transition', result='pass', safekeeper=sk_id, signal=action)

    def flush_quorum(self, record):
        assert re.fullmatch(r'[0-9a-f]{32}', record['tenant']) and re.fullmatch(r'[0-9a-f]{32}', record['timeline'])
        def lsn(value):
            assert re.fullmatch(r'[0-9A-F]+/[0-9A-F]+', value)
            high, low = value.split('/'); return (int(high, 16) << 32) + int(low, 16)
        wanted = lsn(record['lsn'])
        count = 0
        for sk_id in (1, 2, 3):
            if sk_id in self.paused_sks:
                continue
            raw = self.engine('curl', '-fsS', '--max-time', '5',
                              f'http://127.0.0.1:{7675+sk_id}/v1/tenant/{record["tenant"]}/timeline/{record["timeline"]}')
            status = json.loads(raw)
            if lsn(status['flush_lsn']) >= wanted:
                count += 1
        assert count >= 2, 'Acknowledged WAL was not flushed by a safekeeper majority'
        self.report('acknowledged WAL is flushed on an actual safekeeper majority', result='pass', durableCopies=count)

    def respond(self, instruction):
        self.probe.stdin.write((json.dumps({'instruction': instruction}) + '\n').encode())
        self.probe.stdin.flush()

    def reviewed_compute_recovery(self):
        # Abrupt exits deliberately require operator review; automated recovery
        # is a separate launch task. Use the existing native operator command.
        state = json.loads((self.root / 'probe-private.json').read_text())
        tenant = state['project']['id']
        assert re.fullmatch(r'[0-9a-f]{32}', tenant)
        assert len(state['computes']) == 2 and len(state['branches']) == 2
        processes = self.engine('sh', '-c', '''for p in /proc/[0-9]*; do
test -r "$p/comm" || continue
name=$(cat "$p/comm")
case "$name" in compute_ctl|postgres) printf '%s %s\\n' "$name" "$(readlink "$p/cwd")" ;; esac
done''')
        assert not any(line.split(maxsplit=1)[0] == 'compute_ctl' or
                       ('postgres' in line and '/var/lib/briven/endpoints/' in line)
                       for line in processes.splitlines() if line.strip()), 'Compute process remains alive; operator recovery refused'
        for compute in state['computes']:
            identity = compute['id']
            assert re.fullmatch(r'[a-zA-Z0-9_-]{1,80}', identity)
            timeline = next(branch['id'] for branch in state['branches'] if branch['name'] == compute['branch'])
            definition = json.loads(self.engine('cat', f'/var/lib/briven/endpoints/{identity}/endpoint.json'))
            assert definition['endpoint_id'] == identity and definition['tenant_id'] == tenant and definition['timeline_id'] == timeline
            self.engine('briven_local', 'endpoint', 'start', identity, timeout=150)
        self.report('reviewed native recovery starts only the two exact retained fixture computes', result='pass')

    def run_probe(self, action):
        self.probe = subprocess.Popen(['node', str(HERE / 'probe.mjs'), action], env=self.env,
                                      stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                      stderr=subprocess.DEVNULL, bufsize=0)
        selector = selectors.DefaultSelector(); selector.register(self.probe.stdout, selectors.EVENT_READ)
        deadline = time.monotonic() + 600
        buffer = b''
        try:
            while True:
                if time.monotonic() > deadline:
                    raise TimeoutError('Owned SQL probe deadline exceeded')
                if not selector.select(5):
                    if self.probe.poll() is not None:
                        break
                    continue
                chunk = os.read(self.probe.stdout.fileno(), 32768)
                if not chunk:
                    assert not buffer, 'Truncated probe result refused'
                    break
                buffer += chunk
                assert len(buffer) < 65536, 'Unbounded probe output refused'
                while b'\n' in buffer:
                    line, buffer = buffer.split(b'\n', 1)
                    self.handle_record(json.loads(line), action)
            assert self.probe.wait(timeout=15) == 0, 'SQL probe failed'
            self.report('complete SQL acceptance phase', result='pass', phase=action)
        finally:
            selector.close()
            if self.probe.poll() is None:
                self.probe.terminate()
                try:self.probe.wait(timeout=10)
                except subprocess.TimeoutExpired:self.probe.kill();self.probe.wait(timeout=5)
            self.probe = None

    def handle_record(self, record, action):
        assert set(record) <= {'check', 'result', 'phase', 'errorClass', 'sqlstate', 'reason',
                               'marker', 'tenant', 'timeline', 'lsn', 'computeFsync'}
        if 'marker' not in record:
            self.report(record.pop('check'), **record)
            assert record.get('result') == 'pass', 'SQL acceptance assertion failed'
            return
        marker = record['marker']
        if marker == 'verify-quorum-flush':
            self.flush_quorum(record); self.respond('quorum-flush-verified')
        elif marker == 'crash-with-uncommitted-transaction' and action == 'crash-hold':
            self.recovery_since = datetime.datetime.now(datetime.timezone.utc).isoformat()
            self.cmd(['docker', 'kill', '--signal', 'KILL', self.row('engine')['Id']])
            self.respond('engine-crashed')
        elif marker == 'ready-to-pause-one-safekeeper' and action == 'one-safekeeper':
            self.sk_signal(1, 'STOP'); self.respond('one-safekeeper-paused')
        elif marker == 'ready-to-pause-majority' and action == 'quorum':
            self.sk_signal(1, 'STOP'); self.sk_signal(2, 'STOP'); self.respond('majority-paused')
        elif marker == 'restore-majority' and action == 'quorum':
            self.sk_signal(1, 'CONT'); self.sk_signal(2, 'CONT'); self.respond('majority-restored')
        elif marker == 'ready-to-pause-object-store' and action == 'object-outage':
            self.cmd(['docker', 'pause', self.row('minio')['Id']]); self.minio_paused = True
            self.respond('object-store-paused')
        else:
            raise RuntimeError('Unexpected fault marker refused')

    def execute(self):
        assert self.cmd(['docker', 'inspect', 'briven-clean-engine-20261011-dsjaqo-engine-1'], check=False).returncode != 0
        available = int(next(line.split()[1] for line in Path('/proc/meminfo').read_text().splitlines() if line.startswith('MemAvailable:'))) * 1024
        assert available > 5.5 * 1024**3 and shutil.disk_usage(self.root).free > 8 * 1024**3
        self.report('disposable runner capacity and production-container absence', result='pass')
        for image, expected in IMAGES.values():
            self.phase = 'pull-' + image.split('@')[0].split('/')[-1]
            self.cmd(['docker', 'pull', image], timeout=300)
            actual = json.loads(self.cmd(['docker', 'image', 'inspect', image]))[0]
            assert actual['Id'] == expected
        self.report('engine and auxiliary images match actual production immutable identities', result='pass')
        resolved = json.loads(self.cmd([*self.compose, 'config', '--format', 'json']))
        for service in resolved['services'].values():
            assert not service.get('ports') and not service.get('privileged')
        self.started = True
        self.phase = 'start-fixture'
        self.cmd([*self.compose, 'up', '-d', '--pull', 'never'], timeout=300)
        self.wait_healthy()
        for sk_id in (1, 2, 3):
            cmdline = self.engine('cat', f'/proc/{self.engine("cat", f"/var/lib/briven/safekeepers/sk{sk_id}/safekeeper.pid").strip()}/cmdline')
            assert '--disable-sync' not in cmdline and '--no-sync' not in cmdline
        self.report('three actual synchronous safekeepers are running', result='pass')
        self.phase = 'seed-and-branch'
        self.run_probe('seed')
        self.phase = 'graceful-restart'
        self.cmd(['docker', 'restart', '--time', '60', self.row('engine')['Id']], timeout=150)
        self.wait_healthy(); self.run_probe('verify')
        self.report('graceful full-engine restart preserves branches rows vectors and indexes', result='pass')
        self.phase = 'forced-crash'
        self.run_probe('crash-hold')
        self.cmd(['docker', 'start', self.row('engine')['Id']])
        self.wait_healthy(); self.reviewed_compute_recovery(); self.run_probe('verify')
        self.report('abrupt full-engine crash preserves commits and rolls back unfinished work', result='pass')
        self.phase = 'one-safekeeper-unavailable'
        try:self.run_probe('one-safekeeper')
        finally:
            if 1 in self.paused_sks:self.sk_signal(1, 'CONT')
        self.phase = 'quorum-unavailable'
        self.run_probe('quorum')
        self.phase = 'object-store-outage'
        try:self.run_probe('object-outage')
        finally:
            if self.minio_paused:
                self.cmd(['docker', 'unpause', self.row('minio')['Id']]); self.minio_paused = False
        self.cmd(['docker', 'kill', '--signal', 'KILL', self.row('engine')['Id']])
        self.cmd(['docker', 'start', self.row('engine')['Id']])
        self.wait_healthy(); self.reviewed_compute_recovery(); self.run_probe('verify'); self.run_probe('final')
        self.report('object-store outage and second forced crash retain all acknowledged branch data', result='pass')
        self.summary['passed'] = True

    def cleanup(self):
        if self.started:
            for sk_id in list(self.paused_sks):self.sk_signal(sk_id, 'CONT')
            if self.minio_paused:self.cmd(['docker', 'unpause', self.row('minio')['Id']])
            identities = self.cmd(['docker', 'ps', '-aq', '--filter', 'label=com.docker.compose.project=' + self.project]).split()
            for identity in identities:
                row = json.loads(self.cmd(['docker', 'inspect', identity]))[0]
                guard_container(row, self.project, row['Config']['Labels']['com.docker.compose.service'])
            self.cmd([*self.compose, 'stop', '--timeout', '60'], timeout=180)
            assert not self.cmd(['docker', 'ps', '-q', '--filter', 'label=com.docker.compose.project=' + self.project]).strip()
        current = self.cmd(['docker', 'ps', '-q']).split()
        assert all(identity in current for identity in self.baseline)
        self.summary['cleanupPassed'] = True
        self.report('all owned fixture services stopped; pre-existing containers preserved; secret artifacts excluded', result='pass')

    def diagnose(self):
        names = self.cmd(['docker', 'ps', '-aq', '--filter', 'label=com.docker.compose.project=' + self.project]).split()
        for identity in names:
            row = json.loads(self.cmd(['docker', 'inspect', identity]))[0]
            self.report('read-only fixture metadata for failure diagnosis', name=row['Name'],
                        imageReference=row['Config']['Image'], imageId=row['Image'],
                        running=row['State']['Running'], health=row['State'].get('Health',{}).get('Status'),
                        ports=row['HostConfig']['PortBindings'], privileged=row['HostConfig']['Privileged'],
                        pidMode=row['HostConfig'].get('PidMode'), networks=list(row['NetworkSettings']['Networks']),
                        mounts=[{key:mount.get(key) for key in ['Type','Name','Source','Destination','RW']}for mount in row['Mounts']])
        engine = self.row('engine')
        if not engine['State']['Running']:
            return
        args = ['docker', 'logs', '--tail', '1000']
        if self.recovery_since:
            args += ['--since', self.recovery_since]
        logs = self.cmd([*args, engine['Id']], check=False)
        self.report('fixed startup log categories', source='container', signals=startup_signals(logs.stdout + logs.stderr))
        for source, path in {
            'controller-postgres': '/var/lib/briven/storage_controller_db/postgres.log',
            'controller': '/var/lib/briven/storage_controller_1/storage_controller.log',
            'pageserver': '/var/lib/briven/pageserver_1/pageserver.log',
            'safekeeper-1': '/var/lib/briven/safekeepers/sk1/safekeeper.log',
        }.items():
            result = self.cmd(['docker', 'exec', engine['Id'], 'tail', '-n', '100', path], check=False)
            self.report('fixed startup log categories', source=source, signals=startup_signals(result.stdout + result.stderr))
        pid = self.engine('head', '-n', '1', '/var/lib/briven/storage_controller_db/postmaster.pid').strip()
        if pid.isdigit():
            result = self.cmd(['docker', 'exec', engine['Id'], 'readlink', f'/proc/{pid}/exe'], check=False)
            executable = Path(result.stdout.strip()).name
            self.report('controller postmaster PID executable identity', pid=int(pid),
                        executable=executable if executable in {'postgres','safekeeper','pageserver','storage_broker','storage_controller','endpoint_storage','neon_local','briven_local','sh'} else 'absent-or-other')
        for port, path in [(1234,'/ready'), (9898,'/v1/status'), (8787,'/healthz'), (7001,'/v1/status')]:
            result = self.cmd(['docker','exec',engine['Id'],'curl','-s','--max-time','3','-o','/dev/null','-w','%{http_code}',f'http://127.0.0.1:{port}{path}'],check=False)
            self.report('fixed startup endpoint status',port=port,status=result.stdout if re.fullmatch(r'[0-9]{3}',result.stdout) else 'unavailable')


def main():
    os.umask(0o077)
    acceptance = Acceptance()
    try:
        acceptance.execute()
    except Exception as error:
        acceptance.summary['passed'] = False
        reason=str(error) if isinstance(error,AssertionError) and re.fullmatch(r'[a-zA-Z0-9 ,;-]{1,160}',str(error)) else 'details suppressed'
        acceptance.report('database durability acceptance failed', result='fail', phase=acceptance.phase,errorClass=type(error).__name__,reason=reason)
        try:acceptance.diagnose()
        except Exception:pass
    finally:
        try:acceptance.cleanup()
        except Exception as error:
            acceptance.summary['cleanupPassed'] = False
            acceptance.report('fixture cleanup failed', result='fail', errorClass=type(error).__name__)
        (acceptance.root / 'summary.json').write_text(json.dumps(acceptance.summary, indent=2) + '\n')
    if not (acceptance.summary['passed'] and acceptance.summary['cleanupPassed']):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
