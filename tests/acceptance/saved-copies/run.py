"""Exact-source Linux acceptance, using only the owned CI storage fixture."""
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('durability_fixture', HERE.parent / 'postgres-durability/run.py')
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)


class Acceptance(fixture.Acceptance):
    def __init__(self):
        assert os.environ.get('GITHUB_ACTIONS') == 'true' and sys.platform == 'linux'
        source = os.environ['GITHUB_SHA']
        assert re.fullmatch('[0-9a-f]{40}', source)
        image = 'local/briven-engine-acceptance:' + source
        actual = json.loads(subprocess.check_output(['docker', 'image', 'inspect', image], text=True))[0]
        assert actual['Config']['Labels']['org.opencontainers.image.revision'] == source
        assert re.fullmatch('sha256:[0-9a-f]{64}', actual['Id'])
        fixture.ENGINE_SHA = source
        fixture.ENGINE_IMAGE = image
        fixture.IMAGES['engine'] = (image, actual['Id'])
        super().__init__()
        self.summary.update(suite='saved-copy-removal', engineConfigId=actual['Id'])
        self.pid_backups=[]
        overlay = self.root / 'candidate-compose.json'
        overlay.write_text(json.dumps({'services': {name: {'image': image} for name in ['engine', 'certificates']}}))
        self.compose += ['-f', str(overlay)]

    def state(self):
        return json.loads((self.root / 'saved-private.json').read_text())

    def run_probe(self, action):
        self.probe = subprocess.Popen(['node', str(HERE / 'probe.mjs'), action], env=self.env,
                                      stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        deadline = time.monotonic() + 300
        try:
            import selectors
            selector = selectors.DefaultSelector()
            selector.register(self.probe.stdout, selectors.EVENT_READ)
            buffer = b''
            while True:
                if time.monotonic() > deadline:
                    raise TimeoutError('Owned saved-copy probe exceeded its deadline')
                if not selector.select(1):
                    if self.probe.poll() is not None:
                        break
                    continue
                chunk = os.read(self.probe.stdout.fileno(), 16384)
                if not chunk:
                    assert not buffer
                    break
                buffer += chunk
                assert len(buffer) < 65536
                while b'\n' in buffer:
                    raw, buffer = buffer.split(b'\n', 1)
                    record = json.loads(raw)
                    assert set(record) <= {'check', 'result', 'phase', 'errorClass', 'reason', 'marker'}
                    if record.get('marker') == 'hide-owned-pidfiles':
                        self.hide_owned_pids()
                        self.respond('pidfiles-hidden')
                    elif record.get('marker') == 'restore-owned-pidfiles':
                        self.restore_owned_pids()
                        self.respond('pidfiles-restored')
                    elif record.get('marker') == 'removal-dispatched':
                        self.interrupt_storage_cleanup()
                    else:
                        self.report(record.pop('check'), **record)
                        assert record.get('result') == 'pass', 'Saved-copy engine assertion failed'
            assert self.probe.wait(timeout=10) == 0, 'Saved-copy engine SQL probe failed'
        finally:
            selector.close()
            if self.probe.poll() is None:
                self.probe.terminate()
                try: self.probe.wait(timeout=10)
                except subprocess.TimeoutExpired: self.probe.kill(); self.probe.wait(timeout=5)
            self.probe = None

    def hide_owned_pids(self):
        state=self.state()
        identity='ep-'+state['project']['id']+'-'+state['restores'][0]['name']
        assert re.fullmatch('ep-[0-9a-f]{32}-rc-[0-9a-f]{24}',identity)
        directory='/var/lib/briven/endpoints/'+identity
        for name in ['pgdata/postmaster.pid','compute_ctl.pid']:
            path=directory+'/'+name
            pid=self.engine('head','-n','1',path).strip()
            assert pid.isdigit() and int(pid)>1
            argv=self.engine('cat','/proc/'+pid+'/cmdline').split('\0')
            assert (name.startswith('pgdata/') and '-D' in argv and directory+'/pgdata' in argv) or ('--compute-id' in argv and identity in argv)
            self.engine('test','!','-e',path+'.acceptance-backup')
            self.engine('mv',path,path+'.acceptance-backup')
            self.pid_backups.append(path)

    def restore_owned_pids(self):
        for path in self.pid_backups:
            self.engine('test','!','-e',path)
            self.engine('mv',path+'.acceptance-backup',path)
        self.pid_backups=[]

    def interrupt_storage_cleanup(self):
        state = self.state()
        target = state['interrupted']
        tenant = state['project']['id']
        assert re.fullmatch('[0-9a-f]{32}', tenant)
        assert re.fullmatch('sc-[0-9a-f]{24}', target['name'])
        journal = f'/var/lib/briven/reserved-removals/{tenant}/{target["name"]}.json'
        # Wait on this one owned intent file, not Docker inventory/stat polling.
        self.engine('sh', '-c', '''n=0
while test "$n" -lt 120; do
  if test -f "$1"; then case "$(cat "$1")" in *'"phase":"storage_removed"'*) exit 0;; esac; fi
  n=$((n+1)); sleep 0.25
done
exit 1''', 'briven-removal-barrier', journal, timeout=40)
        record = json.loads(self.engine('cat', journal))
        assert record['scope']['timeline_id'] == target['id'] and record['phase'] == 'storage_removed'
        row = self.row('engine')
        self.cmd(['docker', 'kill', '--signal', 'KILL', row['Id']])
        assert not self.row('engine')['State']['Running']
        self.paused_sks.clear()
        self.report('actual engine interruption retains an exact unfinished WAL cleanup intent', result='pass')

    def restart_main(self):
        state = self.state()
        tenant = state['project']['id']
        main = state['computes'][0]
        assert len(state['computes']) == 1 and main['branch'] == 'main'
        identity = main['id']
        assert identity == f'ep-{tenant}'
        assert self.engine('ls', '-1', '/var/lib/briven/endpoints').splitlines() == [identity]
        conf = json.loads(self.engine('cat', f'/var/lib/briven/endpoints/{identity}/endpoint.json'))
        assert conf['tenant_id'] == tenant and conf['endpoint_id'] == identity
        # The full container was killed. No retained compute may still be alive.
        self.engine('sh', '-c', '''for p in /proc/[0-9]*; do
 test -r "$p/comm" || continue
 case "$(cat "$p/comm")" in
  compute_ctl) exit 1;;
  postgres) case "$(readlink "$p/cwd")" in /var/lib/briven/endpoints/*) exit 1;; esac;;
 esac
done''')
        self.engine('env', 'LD_LIBRARY_PATH=/usr/local/v16/lib', 'briven_local', 'endpoint', 'start', identity, '--allow-multiple', timeout=150)
        self.report('reviewed restart starts only the exact preserved main compute', result='pass')

    def storage_absent(self):
        state = self.state()
        tenant = state['project']['id']
        for target in state['copies'] + state['restores']:
            timeline = target['id']
            assert re.fullmatch('[0-9a-f]{32}', timeline)
            self.engine('test', '!', '-e', f'/var/lib/briven/pageserver_1/tenants/{tenant}/timelines/{timeline}')
            for sk in [1, 2, 3]:
                self.engine('test', '!', '-e', f'/var/lib/briven/safekeepers/sk{sk}/{tenant}/{timeline}')
        objects = self.engine('aws', '--endpoint-url', 'http://minio:9000', 's3', 'ls', 's3://briven/', '--recursive', timeout=60)
        assert not any(target['id'] in objects for target in state['copies'] + state['restores'])
        assert self.engine('ls', '-1', '/var/lib/briven/removed-endpoints').strip() == ''
        self.report('all removed local timeline caches and remote storage WAL objects are absent', result='pass')

    def execute(self):
        for service, (image, expected) in fixture.IMAGES.items():
            if service != 'engine': self.cmd(['docker', 'pull', image], timeout=300)
            actual = json.loads(self.cmd(['docker', 'image', 'inspect', image]))[0]
            assert actual['Id'] == expected
        resolved = json.loads(self.cmd([*self.compose, 'config', '--format', 'json']))
        for service in resolved['services'].values():
            assert not service.get('ports') and not service.get('privileged')
            assert service['logging']['options'] == {'max-size': '10m', 'max-file': '3'}
        self.started = True
        self.phase = 'start-isolated-candidate'
        self.cmd([*self.compose, 'up', '-d', '--pull', 'never'], timeout=300)
        self.wait_healthy()
        self.phase = 'seed'
        self.run_probe('seed')
        state = self.state()
        tenant = state['project']['id']
        branch = state['restores'][0]['name']
        identity = f'ep-{tenant}-{branch}'
        assert re.fullmatch('ep-[0-9a-f]{32}-rc-[0-9a-f]{24}', identity)
        self.engine('env', 'LD_LIBRARY_PATH=/usr/local/v16/lib', 'briven_local', 'endpoint', 'create', identity,
                    '--tenant-id', tenant, '--branch-name', branch, '--pg-port', '55443',
                    '--external-http-port', '55444', '--internal-http-port', '55445')
        self.engine('sh', '-c', 'printf "\\nmax_prepared_transactions = 4\\n" >> "$1"', 'briven-owned-test-config',
                    f'/var/lib/briven/endpoints/{identity}/postgresql.conf')
        self.phase = 'real-restore-and-safe-removal'
        self.run_probe('exercise')
        self.phase = 'prepare-interruption'
        self.run_probe('prepare-interruption')
        self.sk_signal(1, 'STOP')
        self.phase = 'interrupt-cleanup'
        self.run_probe('interrupt')
        self.cmd(['docker', 'start', self.row('engine')['Id']])
        self.wait_healthy()
        self.restart_main()
        self.phase = 'restart-reconciliation'
        self.run_probe('final')
        self.storage_absent()
        self.summary['passed'] = True

    def cleanup(self):
        if self.pid_backups:
            self.restore_owned_pids()
        super().cleanup()
        if self.started:
            self.cmd([*self.compose, 'down', '--volumes', '--remove-orphans'], timeout=180)
            assert not self.cmd(['docker', 'ps', '-aq', '--filter', 'label=com.docker.compose.project='+self.project]).strip()
            assert not self.cmd(['docker', 'volume', 'ls', '-q', '--filter', 'label=com.docker.compose.project='+self.project]).strip()
            self.report('only owned CI containers and volumes were removed', result='pass')


def main():
    os.umask(0o077)
    acceptance = Acceptance()
    try:
        acceptance.execute()
    except Exception as error:
        acceptance.summary['passed'] = False
        reason = str(error) if isinstance(error, AssertionError) and re.fullmatch('[a-zA-Z0-9 ,;-]{1,160}', str(error)) else 'details suppressed'
        acceptance.report('saved-copy engine acceptance failed', result='fail', phase=acceptance.phase, errorClass=type(error).__name__, reason=reason)
        try: acceptance.diagnose()
        except Exception: pass
    finally:
        try: acceptance.cleanup()
        except Exception as error:
            acceptance.summary['cleanupPassed'] = False
            acceptance.report('owned fixture cleanup failed', result='fail', errorClass=type(error).__name__)
        (acceptance.root / 'summary.json').write_text(json.dumps(acceptance.summary, indent=2)+'\n')
    if not (acceptance.summary['passed'] and acceptance.summary['cleanupPassed']): raise SystemExit(1)


if __name__ == '__main__': main()
