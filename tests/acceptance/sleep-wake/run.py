"""Real sleep/wake acceptance; faults are restricted to a fresh Linux CI namespace."""
import importlib.util
import json
import os
from pathlib import Path
import re
import selectors
import shutil
import subprocess
import sys
import time
import traceback

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('durability_fixture', HERE.parent / 'postgres-durability/run.py')
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
SOURCE = '07711c126016e0b8c7958a62e5fc95d22a844bd2'
IMAGE = 'ghcr.io/flndrn-dev/briven-engine@sha256:359ca9530ceb35f77ee83d7dea1b1d409ccd9647810cd97df35843ba8ab3bbdc'
CONFIG = 'sha256:94578897365a549c114be6578de1046807612954b913f699d70fad283a9231c6'
BARRIERS = {'before-stop', 'after-stop', 'before-start', 'after-start'}
SIGNALS = {'STOP', 'CONT', 'KILL'}
RUNTIME = '/tmp/briven-lifecycle-acceptance'


def endpoint_id(tenant):
    assert re.fullmatch('[0-9a-f]{32}', tenant), 'Invalid owned main endpoint'
    return 'ep-' + tenant


def guard_api(argv, parent):
    assert argv in (['/usr/local/bin/briven_control_api'], ['briven_control_api']) and parent in (0, 1), 'Unbound API process refused'


class Acceptance(fixture.Acceptance):
    def __init__(self):
        global SOURCE, IMAGE, CONFIG
        if os.environ.get('BRIVEN_SLEEP_WAKE_CANDIDATE') == 'true':
            assert os.environ.get('GITHUB_ACTIONS') == 'true' and sys.platform == 'linux'
            SOURCE = os.environ['GITHUB_SHA']
            assert re.fullmatch('[0-9a-f]{40}', SOURCE)
            IMAGE = 'local/briven-engine-acceptance:' + SOURCE
            image = json.loads(subprocess.check_output(['docker', 'image', 'inspect', IMAGE], text=True))[0]
            assert image['Config']['Labels']['org.opencontainers.image.revision'] == SOURCE
            CONFIG = image['Id']
            assert re.fullmatch('sha256:[0-9a-f]{64}', CONFIG)
        fixture.ENGINE_SHA, fixture.ENGINE_IMAGE = SOURCE, IMAGE
        fixture.IMAGES['engine'] = (IMAGE, CONFIG)
        super().__init__(fixture_attempt=os.environ['GITHUB_RUN_ATTEMPT'] + '1' if os.environ.get('BRIVEN_SLEEP_WAKE_CANDIDATE') == 'true' else None)
        self.summary.update(suite='database-restart-diagnostic' if os.environ.get('BRIVEN_SLEEP_WAKE_RESTART_ONLY') == 'true' else 'database-sleep-wake', engineConfigId=CONFIG)
        self.shell_paused = False
        self.paused_postmaster = None
        self.hidden_pid = None
        self.draining_client = None
        self.orphan_controller = None
        self.orphan_backup = None
        self.capacity_server = None
        self.sleep_launches = None
        self.binding_backup = None
        overlay = self.root / 'candidate-compose.json'
        overlay.write_text(json.dumps({'services': {name: {'image': IMAGE} for name in ['engine', 'certificates']}}))
        self.compose += ['-f', str(overlay)]

    def state(self):
        return json.loads((self.root / 'lifecycle-private.json').read_text())

    def endpoint(self):
        return endpoint_id(self.state()['project']['id'])

    def identity(self, pid):
        assert str(pid).isdigit() and int(pid) > 0
        argv = self.engine('cat', f'/proc/{pid}/cmdline').split('\0')
        argv = [value for value in argv if value]
        stat = self.engine('cat', f'/proc/{pid}/stat').rsplit(') ', 1)[1].split()
        return argv, int(stat[1])

    def signal(self, pid, signal):
        assert signal in SIGNALS and str(pid).isdigit() and int(pid) > 0
        self.engine('sh', '-c', 'kill "-$1" "$2"', 'briven-owned-signal', signal, str(pid))

    def processes(self):
        endpoint = self.endpoint()
        result = {}
        for name, suffix in [('postgres', 'pgdata/postmaster.pid'), ('controller', 'compute_ctl.pid')]:
            path = f'/var/lib/briven/endpoints/{endpoint}/{suffix}'
            raw = self.cmd(['docker', 'exec', self.row('engine')['Id'], 'head', '-n', '1', path], check=False)
            if raw.returncode or not raw.stdout.strip().isdigit():
                result[name] = None
                continue
            pid = int(raw.stdout.strip())
            exists = self.cmd(['docker', 'exec', self.row('engine')['Id'], 'test', '-r', f'/proc/{pid}/cmdline'], check=False)
            if exists.returncode:
                result[name] = None
                continue
            argv, _ = self.identity(pid)
            status = self.engine('cat', f'/proc/{pid}/stat').rsplit(') ', 1)[1].split()[0]
            if status in ('Z', 'X'):
                result[name] = None
                continue
            if name == 'postgres':
                assert Path(argv[0]).name == 'postgres' and ['-D', f'/var/lib/briven/endpoints/{endpoint}/pgdata'] == argv[argv.index('-D'):argv.index('-D')+2]
            else:
                assert Path(argv[0]).name == 'compute_ctl' and endpoint in argv
            result[name] = pid
        return result

    def census(self):
        raw = self.engine('sh', '-c', 'n=0; for p in /proc/[0-9]*; do n=$((n+1)); test "$n" -le 16384 || exit 1; e=$(readlink "$p/exe" 2>/dev/null) || continue; case "$e" in /usr/local/bin/compute_ctl|/usr/local/v17/bin/postgres) printf "%s\\n" "${p##*/}";; esac; done')
        matches = []
        for pid in raw.splitlines():
            assert pid.isdigit() and int(pid) > 1
            result = self.cmd(['docker', 'exec', self.row('engine')['Id'], 'cat', f'/proc/{pid}/cmdline'], check=False)
            if result.returncode: continue
            argv = [v for v in result.stdout.split('\0') if v]
            if not argv: continue
            kind = Path(argv[0]).name
            directory = '/var/lib/briven/endpoints/' + self.endpoint() + '/pgdata'
            bound = (kind == 'postgres' and ['-D', directory] in [argv[n:n+2] for n in range(len(argv)-1)]) or (kind == 'compute_ctl' and ['--compute-id', self.endpoint()] in [argv[n:n+2] for n in range(len(argv)-1)] and ['--pgdata', directory] in [argv[n:n+2] for n in range(len(argv)-1)])
            if bound: matches.append({'kind': kind, 'pid': int(pid)})
        return matches

    def launches(self):
        result = self.cmd(['docker', 'exec', self.row('engine')['Id'], 'cat', RUNTIME + '/actual-start-invocations'], check=False)
        if result.returncode: return 0
        lines = result.stdout.splitlines()
        assert all(value == self.endpoint() for value in lines)
        return len(lines)

    def restart_api(self, barrier=False, capacity=False, scheduler=True):
        row = self.row('engine')
        if not self.shell_paused:
            argv, _ = self.identity(1)
            assert argv == ['/bin/sh', '/start-private.sh'], 'Owned shell supervisor identity differs'
            # Namespace init ignores unhandled STOP sent from inside its own
            # namespace. Docker delivers it from the verified ancestor namespace.
            self.cmd(['docker', 'kill', '--signal', 'STOP', row['Id']])
            assert self.engine('cat', '/proc/1/stat').rsplit(') ', 1)[1].split()[0] in ('T', 't'), 'Owned supervisor did not actually stop'
            self.shell_paused = True
        pid = int(self.engine('cat', '/var/lib/briven/briven-control-api.pid').strip())
        guard_api(*self.identity(pid))
        assert self.engine('readlink', f'/proc/{pid}/exe').strip() == '/usr/local/bin/briven_control_api'
        self.signal(pid, 'KILL')
        if barrier:
            wrapper = int(self.engine('cat', RUNTIME + '/barrier-pid').strip())
            argv, _ = self.identity(wrapper)
            assert '/tmp/lifecycle-cli.sh' in argv and self.endpoint() in argv, 'Owned CLI barrier identity differs'
            self.signal(wrapper, 'KILL')
        self.mode('none')
        self.cmd(['docker', 'exec', '-d', row['Id'], 'sh', '-c',
                  'exec env LD_LIBRARY_PATH=/usr/local/v16/lib BRIVEN_LOCAL_BIN=/tmp/lifecycle-cli.sh '
                  'BRIVEN_COMPUTE_LIFECYCLE_ENABLED="$3" BRIVEN_COMPUTE_LIFECYCLE_ENDPOINTS_JSON="$1" '
                  'BRIVEN_COMPUTE_RESOURCE_SOCKET="$2" briven_control_api >/tmp/lifecycle-api.log 2>&1', 'briven-owned-api', json.dumps([self.endpoint()]), '/run/briven-compute-control/control.sock' if capacity else '', 'true' if scheduler else 'false'])
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            result = self.cmd(['docker', 'exec', row['Id'], 'curl', '-fsS', '--max-time', '1', 'http://127.0.0.1:8787/healthz'], check=False)
            if result.returncode == 0:
                new = int(self.engine('cat', '/var/lib/briven/briven-control-api.pid').strip())
                guard_api(*self.identity(new))
                assert self.engine('readlink', f'/proc/{new}/exe').strip() == '/usr/local/bin/briven_control_api'
                assert new != pid
                self.report('actual API binary restarted with retained catalog and single endpoint allowlist', result='pass')
                return
            time.sleep(0.2)
        raise AssertionError('Owned replacement API did not become healthy')

    def mode(self, value):
        assert value == 'none' or value in BARRIERS
        self.engine('sh', '-c', 'rm -f "$1/reached" "$1/release" "$1/barrier-pid"; printf "%s\\n" "$2" > "$1/mode"', 'briven-owned-barrier', RUNTIME, value)

    def wait_barrier(self, value):
        assert value in BARRIERS or value == 'prepare-after-stop'
        # The marker is a single owned file. No Docker inventory/stat polling.
        self.engine('sh', '-c', 'n=0; while test "$n" -lt 1000; do if test -f "$1/reached" && test "$(cat "$1/reached")" = "$2"; then exit 0; fi; n=$((n+1)); sleep 0.1; done; exit 1', 'briven-owned-barrier', RUNTIME, value, timeout=110)

    def record(self):
        tenant = self.state()['project']['id']
        sql = "SELECT record FROM briven_compute_lifecycle.endpoints WHERE endpoint_id='" + endpoint_id(tenant) + "'"
        return json.loads(self.cmd(['docker', 'exec', self.row('catalog')['Id'], 'psql', '-U', 'briven_catalog', '-d', 'briven_control', '-tAc', sql]))

    def drain_query(self, sql, expected):
        self.draining_client.stdin.write((sql + '\n').encode()); self.draining_client.stdin.flush()
        selector = selectors.DefaultSelector(); selector.register(self.draining_client.stdout, selectors.EVENT_READ)
        try:
            assert selector.select(10), 'Owned draining client query deadline exceeded'
            assert self.draining_client.stdout.readline().decode().strip() == expected, 'Draining client result differs'
        finally:
            selector.close()

    def hold_draining_session(self):
        definition = json.loads(self.engine('cat', f'/var/lib/briven/endpoints/{self.endpoint()}/endpoint.json'))
        port = definition['pg_port'] if 'pg_port' in definition else definition['pg_address'].rsplit(':', 1)[1]
        assert str(port).isdigit()
        self.draining_client = subprocess.Popen(['docker', 'exec', '-i', self.row('engine')['Id'], 'env', 'LD_LIBRARY_PATH=/usr/local/v17/lib', '/usr/local/v17/bin/psql', '-XAtq', '-v', 'ON_ERROR_STOP=1', '-h', '127.0.0.1', '-p', str(port), '-U', 'cloud_admin', '-d', 'postgres'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.drain_query("BEGIN; SELECT 'held';", 'held')

    def finish_draining_session(self):
        self.drain_query("INSERT INTO lifecycle_records VALUES(1000,'draining commit','[1,2,3]'); COMMIT; SELECT 'committed';", 'committed')
        self.draining_client.stdin.close()
        assert self.draining_client.wait(timeout=10) == 0, 'Graceful drain interrupted an existing transaction'
        self.draining_client = None
        self.report('real existing transaction commits after smart dispatch and API restart', result='pass')

    def handle(self, message):
        marker = message['marker']
        endpoint = self.endpoint()
        if marker == 'enable-config':
            self.engine('mkdir', '-p', RUNTIME)
            installed = subprocess.run(['docker', 'exec', '-i', self.row('engine')['Id'], 'sh', '-c',
                                        'umask 077; cat > /tmp/lifecycle-cli.sh; chmod 700 /tmp/lifecycle-cli.sh'],
                                       input=(HERE / 'cli-barrier.sh').read_text(), capture_output=True, text=True, timeout=15)
            assert installed.returncode == 0, 'Owned CLI barrier install failed'
            self.engine('sh', '-c', 'printf "%s\\n" "$2" > "$1/target"', 'briven-owned-target', RUNTIME, endpoint)
            # Prepared transactions are enabled only in this disposable fixture.
            self.engine('env', 'LD_LIBRARY_PATH=/usr/local/v16/lib', 'briven_local', 'endpoint', 'stop', endpoint, '--mode', 'smart', '--no-wait')
            for _ in range(120):
                if all(value is None for value in self.processes().values()): break
                time.sleep(0.25)
            assert all(value is None for value in self.processes().values())
            self.engine('sh', '-c', 'printf "\\nmax_prepared_transactions = 4\\n" >> "$1"', 'briven-owned-config', f'/var/lib/briven/endpoints/{endpoint}/postgresql.conf')
            self.engine('env', 'LD_LIBRARY_PATH=/usr/local/v16/lib', 'briven_local', 'endpoint', 'start', endpoint, timeout=150)
            self.restart_api()
        elif marker == 'restart-api':
            before = self.processes()
            self.restart_api()
            assert self.processes() == before, 'API restart changed compute identities'
        elif marker == 'inspect-sleep':
            assert not self.census(), 'Sleeping endpoint retains a bound process'
            self.sleep_launches = self.launches()
            self.report('sleeping endpoint has no bound PostgreSQL or compute controller', result='pass')
        elif marker == 'inspect-running':
            census = self.census()
            assert len(census) == 2 and {v['kind'] for v in census} == {'postgres', 'compute_ctl'}, 'Concurrent wake created duplicate processes'
            assert self.sleep_launches is not None and self.launches() == self.sleep_launches + 1, 'Concurrent wake invoked actual CLI start more than once'
            self.report('independent process census and real CLI invocation count prove one cold startup', result='pass')
        elif marker == 'enable-capacity-refusal':
            assert self.record()['phase'] == 'sleeping' and not self.census()
            host_pid = self.row('engine')['State']['Pid']; assert host_pid > 1
            self.capacity_server = subprocess.Popen(['sudo', '--preserve-env=GITHUB_ACTIONS,GITHUB_RUN_ID,GITHUB_RUN_ATTEMPT', sys.executable, '-B', str(HERE / 'capacity-refusal.py'), str(host_pid), self.state()['project']['id'], self.row('engine')['Name'].lstrip('/'), CONFIG], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            selector = selectors.DefaultSelector(); selector.register(self.capacity_server.stdout, selectors.EVENT_READ)
            try:
                assert selector.select(10), 'Owned capacity refusal fixture startup deadline exceeded'
                assert self.capacity_server.stdout.readline().decode().strip() == 'owned-capacity-refusal-ready'
            finally: selector.close()
            self.restart_api(capacity=True)
            self.sleep_launches = self.launches()
        elif marker == 'disable-capacity-refusal':
            assert not self.census() and self.launches() == self.sleep_launches, 'Capacity refusal allocated a process'
            self.restart_api()
            self.capacity_server.stdin.close(); assert self.capacity_server.wait(timeout=10) == 0
            responses = [json.loads(line) for line in self.capacity_server.stdout.read().decode().splitlines()]
            admitted = [r for r in responses if r.get('action') == 'admit']
            assert len(admitted) == 1 and admitted[0] == {'event': 'owned-resource-response', 'action': 'admit', 'project': self.state()['project']['id'], 'negativeDelivered': True}, 'Exact admission refusal was not delivered'
            self.report('exact valid resource admission request received and negative response delivered', result='pass')
            self.capacity_server = None
        elif marker == 'tamper-binding':
            self.restart_api(scheduler=False)
            self.binding_backup = self.record()
            altered = dict(self.binding_backup, timelineId='f' * 32)
            body = json.dumps(altered).replace("'", "''")
            sql = "UPDATE briven_compute_lifecycle.endpoints SET timeline_id='" + 'f' * 32 + "',record='" + body + "' WHERE endpoint_id='" + endpoint + "'"
            self.cmd(['docker', 'exec', self.row('catalog')['Id'], 'psql', '-v', 'ON_ERROR_STOP=1', '-U', 'briven_catalog', '-d', 'briven_control', '-c', sql])
        elif marker == 'restore-binding':
            assert self.record() == dict(self.binding_backup, timelineId='f' * 32), 'Refused binding mutation changed durable state'
            body = json.dumps(self.binding_backup).replace("'", "''")
            sql = "UPDATE briven_compute_lifecycle.endpoints SET timeline_id='" + self.binding_backup['timelineId'] + "',record='" + body + "' WHERE endpoint_id='" + endpoint + "'"
            self.cmd(['docker', 'exec', self.row('catalog')['Id'], 'psql', '-v', 'ON_ERROR_STOP=1', '-U', 'briven_catalog', '-d', 'briven_control', '-c', sql])
            self.binding_backup = None
            self.restart_api()
        elif marker == 'arm-orphan-controller':
            self.orphan_controller = self.processes()['controller']
            assert self.orphan_controller
            conf = json.loads(self.engine('cat', f'/var/lib/briven/endpoints/{endpoint}/endpoint.json'))
            for attempt in range(5):
                self.signal(self.orphan_controller, 'STOP')
                sql = "SELECT count(*) FROM pg_stat_activity WHERE backend_type='client backend' AND pid<>pg_backend_pid()"
                count = self.engine('env', 'LD_LIBRARY_PATH=/usr/local/v17/lib', '/usr/local/v17/bin/psql', '-XAtw', '-h', '127.0.0.1', '-p', str(conf['pg_port']), '-U', 'cloud_admin', '-d', 'postgres', '-c', sql).strip()
                if count == '0': break
                self.signal(self.orphan_controller, 'CONT'); time.sleep(1)
            assert count == '0', 'Owned controller must be paused between ephemeral monitor connections'
        elif marker == 'hide-orphan-controller':
            assert self.processes()['postgres'] is None and self.processes()['controller'] == self.orphan_controller
            path = f'/var/lib/briven/endpoints/{endpoint}/compute_ctl.pid'
            self.engine('mv', path, path + '.orphan-backup'); self.orphan_backup = path
            self.report('actual stopped PostgreSQL has a live paused controller with missing PID file', result='pass')
        elif marker == 'restore-orphan-controller':
            path = self.orphan_backup
            exists = self.cmd(['docker', 'exec', self.row('engine')['Id'], 'test', '-e', path], check=False)
            if exists.returncode: self.engine('mv', path + '.orphan-backup', path)
            self.orphan_backup = None
            argv, _ = self.identity(self.orphan_controller)
            assert Path(argv[0]).name == 'compute_ctl' and endpoint in argv
            self.signal(self.orphan_controller, 'CONT'); self.orphan_controller = None
        elif marker == 'pause-postmaster':
            self.paused_postmaster = self.processes()['postgres']
            assert self.paused_postmaster
            self.signal(self.paused_postmaster, 'STOP')
        elif marker == 'resume-postmaster':
            self.signal(self.paused_postmaster, 'CONT'); self.paused_postmaster = None
        elif marker.startswith('arm-'):
            self.mode(marker[4:])
        elif marker.startswith('interrupt-'):
            mode = marker[len('interrupt-'):]
            if mode == 'after-stop':
                self.wait_barrier('prepare-after-stop')
                self.hold_draining_session()
                self.engine('touch', RUNTIME + '/release')
            self.wait_barrier(mode)
            row = self.record()
            assert row['phase'] == ('stopping' if mode.endswith('stop') else 'starting')
            if mode in ('before-stop', 'after-stop'): assert row['stopRequested'] is False
            if mode == 'before-start': assert all(value is None for value in self.processes().values())
            if mode == 'after-start': assert all(value is not None for value in self.processes().values())
            self.report('real CLI interrupted after durable transition', result='pass', boundary=mode, phase=row['phase'])
            self.restart_api(barrier=True)
            if mode == 'after-stop':
                assert self.record()['phase'] == 'stopping' and all(value is not None for value in self.processes().values())
                self.finish_draining_session()
        elif marker == 'crash-sleeping-engine':
            assert self.record()['phase'] == 'sleeping' and all(value is None for value in self.processes().values())
            self.cmd(['docker', 'kill', '--signal', 'KILL', self.row('engine')['Id']])
            self.shell_paused = False
            self.cmd(['docker', 'start', self.row('engine')['Id']])
            self.wait_healthy()
            assert self.record()['phase'] == 'sleeping' and all(value is None for value in self.processes().values())
            self.restart_api()
        elif marker == 'crash-owned-compute':
            before = self.processes()
            self.signal(before['postgres'], 'KILL')
        elif marker == 'reviewed-compute-start':
            for _ in range(120):
                if all(value is None for value in self.processes().values()): break
                time.sleep(0.25)
            assert all(value is None for value in self.processes().values())
            assert not self.census(), 'Reviewed start refuses an orphan or duplicate process'
            self.engine('env', 'LD_LIBRARY_PATH=/usr/local/v16/lib', 'briven_local', 'endpoint', 'start', endpoint, '--allow-multiple', timeout=150)
        elif marker == 'hide-controller-identity':
            path = f'/var/lib/briven/endpoints/{endpoint}/compute_ctl.pid'
            self.hidden_pid = path
            self.engine('cp', path, path + '.acceptance-backup')
            self.engine('sh', '-c', 'printf "1\\n" > "$1"', 'briven-owned-ambiguity', path)
        elif marker == 'restore-controller-identity':
            self.engine('mv', self.hidden_pid + '.acceptance-backup', self.hidden_pid); self.hidden_pid = None
        else:
            raise AssertionError('Unknown fault marker refused')
        self.respond(marker)

    def run_probe(self):
        self.probe = subprocess.Popen(['node', str(HERE / 'probe.mjs')], env=self.env, stdin=subprocess.PIPE,
                                      stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, bufsize=0)
        selector = selectors.DefaultSelector(); selector.register(self.probe.stdout, selectors.EVENT_READ)
        deadline = time.monotonic() + 2400
        buffer = b''
        try:
            while True:
                assert time.monotonic() < deadline, 'Owned lifecycle probe deadline exceeded'
                if not selector.select(1):
                    if self.probe.poll() is not None: break
                    continue
                chunk = os.read(self.probe.stdout.fileno(), 32768)
                if not chunk: assert not buffer; break
                buffer += chunk
                assert len(buffer) < 65536
                while b'\n' in buffer:
                    raw, buffer = buffer.split(b'\n', 1)
                    record = json.loads(raw)
                    assert set(record) <= {'check', 'result', 'phase', 'marker', 'errorClass', 'reason', 'sqlstate', 'cycles', 'checksum', 'starts'}
                    if 'marker' in record: self.handle(record)
                    else:
                        self.report(record.pop('check'), **record)
                        assert record.get('result') == 'pass', 'Actual sleep wake assertion failed'
            assert self.probe.wait(timeout=15) == 0, 'Actual lifecycle probe failed'
        finally:
            selector.close()
            if self.probe.poll() is None:
                self.probe.terminate()
                try: self.probe.wait(timeout=10)
                except subprocess.TimeoutExpired: self.probe.kill(); self.probe.wait(timeout=5)
            self.probe = None

    def execute(self):
        assert self.cmd(['docker', 'inspect', 'briven-clean-engine-20261011-dsjaqo-engine-1'], check=False).returncode != 0
        available = int(next(line.split()[1] for line in Path('/proc/meminfo').read_text().splitlines() if line.startswith('MemAvailable:'))) * 1024
        assert available > 5.5 * 1024**3 and shutil.disk_usage(self.root).free > 8 * 1024**3
        for image, expected in fixture.IMAGES.values():
            if not image.startswith('local/briven-engine-acceptance:'):
                self.cmd(['docker', 'pull', image], timeout=300)
            assert json.loads(self.cmd(['docker', 'image', 'inspect', image]))[0]['Id'] == expected
        self.started = True
        self.phase = 'start-immutable-fixture'
        self.cmd([*self.compose, 'up', '-d', '--pull', 'never'], timeout=300)
        self.wait_healthy()
        self.phase = 'actual-lifecycle-acceptance'
        self.run_probe()
        self.summary['passed'] = True

    def cleanup(self):
        errors = []
        def attempt(action):
            try: action()
            except Exception as error: errors.append(type(error).__name__)
        def close_drain():
            if self.draining_client and self.draining_client.poll() is None:
                self.draining_client.stdin.close()
                try: self.draining_client.wait(timeout=10)
                except subprocess.TimeoutExpired: self.draining_client.kill(); self.draining_client.wait(timeout=5)
        def close_capacity():
            if self.capacity_server and self.capacity_server.poll() is None:
                self.capacity_server.stdin.close()
                assert self.capacity_server.wait(timeout=10) == 0
        def restore_processes():
            result = self.cmd(['docker', 'inspect', self.project + '-engine-1'], check=False)
            if result.returncode: return
            row = json.loads(result.stdout)[0]; fixture.guard_container(row, self.project, 'engine')
            if not row['State']['Running']: return
            if self.orphan_controller:
                argv, _ = self.identity(self.orphan_controller)
                assert Path(argv[0]).name == 'compute_ctl' and self.endpoint() in argv
                self.signal(self.orphan_controller, 'CONT')
            if self.paused_postmaster: self.signal(self.paused_postmaster, 'CONT')
            if self.hidden_pid: self.engine('mv', self.hidden_pid + '.acceptance-backup', self.hidden_pid)
            if self.shell_paused: self.cmd(['docker', 'kill', '--signal', 'CONT', row['Id']])
        def remove_namespace():
            identities = self.cmd(['docker', 'ps', '-aq', '--filter', 'label=com.docker.compose.project=' + self.project]).split()
            for identity in identities:
                row = json.loads(self.cmd(['docker', 'inspect', identity]))[0]
                fixture.guard_container(row, self.project, row['Config']['Labels']['com.docker.compose.service'])
            self.cmd([*self.compose, 'down', '--timeout', '60', '--volumes', '--remove-orphans'], timeout=180)
            for kind, flag in [('ps', '-aq'), ('volume', 'ls'), ('network', 'ls')]:
                args = ['docker', kind, flag] + ([] if kind == 'ps' else ['-q'])
                assert not self.cmd([*args, '--filter', 'label=com.docker.compose.project=' + self.project]).strip()
        try:
            attempt(close_drain)
            attempt(close_capacity)
            if self.started:
                attempt(restore_processes)
                attempt(remove_namespace)
            def preserve_baseline():
                current = self.cmd(['docker', 'ps', '-q']).split()
                assert all(identity in current for identity in self.baseline)
            attempt(preserve_baseline)
        finally:
            for path in self.root.glob('*private*'): attempt(path.unlink)
            if list(self.root.glob('*private*')): errors.append('PrivateFileCleanupIncomplete')
        self.summary['cleanupPassed'] = not errors
        if errors:
            self.report('independent fixture cleanup attempts completed with errors', result='fail', errorClasses=errors)
            raise AssertionError('Owned fixture cleanup incomplete')
        self.report('owned fixture processes containers volumes networks and private files removed', result='pass')

    def diagnose(self):
        row = self.row('engine')
        self.report('owned engine terminal state', running=row['State']['Running'], exitCode=row['State']['ExitCode'], oomKilled=row['State'].get('OOMKilled'), finishedAt=row['State'].get('FinishedAt'))
        if row['State']['Running'] and (self.root / 'lifecycle-private.json').exists():
            try:
                record = self.record()
                self.report('owned lifecycle timeout state', record=record, census=self.census(), now=int(time.time()))
                api = self.engine('cat', '/var/lib/briven/briven-control-api.pid').strip()
                values = self.engine('cat', '/proc/' + api + '/environ').split('\0')
                selected = dict(v.split('=', 1) for v in values if v.startswith(('BRIVEN_COMPUTE_LIFECYCLE_ENABLED=', 'BRIVEN_COMPUTE_LIFECYCLE_ENDPOINTS_JSON=', 'BRIVEN_COMPUTE_RESOURCE_SOCKET=')))
                self.report('owned actual API lifecycle flags', flags=selected)
                definition = json.loads(self.engine('cat', '/var/lib/briven/endpoints/' + self.endpoint() + '/endpoint.json'))
                sql = "SELECT json_build_object('clients',count(*) FILTER(WHERE backend_type='client backend' AND pid<>pg_backend_pid()),'prepared',(SELECT count(*) FROM pg_prepared_xacts),'monitor',count(*) FILTER(WHERE application_name='compute_ctl:compute_monitor'),'inspection',count(*) FILTER(WHERE application_name='briven_lifecycle_inspection' AND pid<>pg_backend_pid()),'customer',count(*) FILTER(WHERE usename='briven_" + self.state()['project']['id'] + "'),'admin',count(*) FILTER(WHERE usename='cloud_admin' AND pid<>pg_backend_pid())) FROM pg_stat_activity"
                activity = json.loads(self.engine('env', 'LD_LIBRARY_PATH=/usr/local/v17/lib', '/usr/local/v17/bin/psql', '-XAtw', '-h', '127.0.0.1', '-p', str(definition['pg_port']), '-U', 'cloud_admin', '-d', 'postgres', '-c', sql))
                self.report('owned lifecycle timeout actual backend counts', activity=activity)
            except Exception as error:
                self.report('owned lifecycle diagnostic unavailable', errorClass=type(error).__name__)
        super().diagnose()


def main():
    os.umask(0o077)
    acceptance = Acceptance()
    try:
        acceptance.execute()
    except Exception as error:
        acceptance.summary['passed'] = False
        reason = str(error) if isinstance(error, AssertionError) and re.fullmatch('[a-zA-Z0-9 ,;-]{1,160}', str(error)) else 'details suppressed'
        frames = [frame for frame in traceback.extract_tb(error.__traceback__) if frame.filename == __file__]
        acceptance.report('database sleep wake acceptance failed', result='fail', phase=acceptance.phase, errorClass=type(error).__name__, reason=reason, guardLine=frames[-1].lineno if frames else None)
        try: acceptance.diagnose()
        except Exception: pass
    finally:
        try: acceptance.cleanup()
        except Exception as error:
            acceptance.summary['cleanupPassed'] = False
            acceptance.report('owned fixture cleanup failed', result='fail', errorClass=type(error).__name__)
        (acceptance.root / 'summary.json').write_text(json.dumps(acceptance.summary, indent=2) + '\n')
    if not (acceptance.summary['passed'] and acceptance.summary['cleanupPassed']): raise SystemExit(1)


if __name__ == '__main__': main()
