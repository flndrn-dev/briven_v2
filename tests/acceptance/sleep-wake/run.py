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
        self.summary.update(suite='database-sleep-wake', engineConfigId=CONFIG)
        self.shell_paused = False
        self.paused_postmaster = None
        self.hidden_pid = None
        self.draining_client = None
        self.orphan_controller = None
        self.orphan_backup = None
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

    def restart_api(self, barrier=False):
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
                  'BRIVEN_COMPUTE_LIFECYCLE_ENABLED=true BRIVEN_COMPUTE_LIFECYCLE_ENDPOINTS_JSON="$1" '
                  'briven_control_api >/tmp/lifecycle-api.log 2>&1', 'briven-owned-api', json.dumps([self.endpoint()])])
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
            assert all(value is None for value in self.processes().values()), 'Sleeping endpoint retains a process'
            self.report('sleeping endpoint has no bound PostgreSQL or compute controller', result='pass')
        elif marker == 'inspect-running':
            assert all(value is not None for value in self.processes().values())
        elif marker == 'arm-orphan-controller':
            self.orphan_controller = self.processes()['controller']
            assert self.orphan_controller
            self.signal(self.orphan_controller, 'STOP')
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
        if self.draining_client and self.draining_client.poll() is None:
            self.draining_client.stdin.close()
            try: self.draining_client.wait(timeout=10)
            except subprocess.TimeoutExpired: self.draining_client.kill(); self.draining_client.wait(timeout=5)
            self.draining_client = None
        if self.started:
            row = self.row('engine')
            if row['State']['Running']:
                if self.orphan_controller:
                    argv, _ = self.identity(self.orphan_controller)
                    assert Path(argv[0]).name == 'compute_ctl' and self.endpoint() in argv
                    self.signal(self.orphan_controller, 'CONT'); self.orphan_controller = None
                if self.paused_postmaster:
                    self.signal(self.paused_postmaster, 'CONT'); self.paused_postmaster = None
                if self.hidden_pid:
                    self.engine('mv', self.hidden_pid + '.acceptance-backup', self.hidden_pid); self.hidden_pid = None
                if self.shell_paused:
                    self.cmd(['docker', 'kill', '--signal', 'CONT', row['Id']]); self.shell_paused = False
            super().cleanup()
            self.cmd([*self.compose, 'down', '--volumes', '--remove-orphans'], timeout=180)
            assert not self.cmd(['docker', 'ps', '-aq', '--filter', 'label=com.docker.compose.project=' + self.project]).strip()
            assert not self.cmd(['docker', 'volume', 'ls', '-q', '--filter', 'label=com.docker.compose.project=' + self.project]).strip()
            assert not self.cmd(['docker', 'network', 'ls', '-q', '--filter', 'label=com.docker.compose.project=' + self.project]).strip()
        for path in self.root.glob('*private*'): path.unlink()
        assert not list(self.root.glob('*private*'))
        self.report('owned fixture processes containers volumes networks and private files removed', result='pass')
        self.summary['cleanupPassed'] = True

    def diagnose(self):
        row = self.row('engine')
        self.report('owned engine terminal state', running=row['State']['Running'], exitCode=row['State']['ExitCode'], oomKilled=row['State'].get('OOMKilled'), finishedAt=row['State'].get('FinishedAt'))
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
