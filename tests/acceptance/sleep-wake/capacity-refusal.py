"""Root-owned negative resource-service fixture; never admits or launches a process."""
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys
import threading

assert os.geteuid() == 0 and os.environ.get('GITHUB_ACTIONS') == 'true'
pid, project, container, image = sys.argv[1:]
assert pid.isdigit() and int(pid) > 1 and re.fullmatch('[0-9a-f]{32}', project)
assert re.fullmatch('sha256:[0-9a-f]{64}', image)
prefix = 'briven-durability-' + os.environ['GITHUB_RUN_ID'] + '-'
assert container in [prefix + os.environ['GITHUB_RUN_ATTEMPT'] + '-engine-1', prefix + os.environ['GITHUB_RUN_ATTEMPT'] + '1-engine-1']
row = json.loads(subprocess.check_output(['docker', 'inspect', '--format', '{"pid":{{.State.Pid}},"image":{{json .Image}},"project":{{json (index .Config.Labels "com.docker.compose.project")}},"purpose":{{json (index .Config.Labels "briven.acceptance.purpose")}}}', container], text=True))
assert row == {'pid': int(pid), 'image': image, 'project': container.removesuffix('-engine-1'), 'purpose': 'postgres-durability'}
root = Path('/proc') / pid / 'root/run/briven-compute-control'
assert not root.exists()
root.mkdir(mode=0o750); os.chown(root, 0, 1000)
path = root / 'control.sock'
server = socket.socket(socket.AF_UNIX)
server.bind(str(path)); os.chown(path, 0, 1000); os.chmod(path, 0o660)
server.listen(4); server.settimeout(0.5)
done = threading.Event()


def serve():
    while not done.is_set():
        try: connection, _ = server.accept()
        except TimeoutError: continue
        with connection:
            connection.settimeout(2)
            data = b''
            while not data.endswith(b'\n') and len(data) <= 1024:
                part = connection.recv(1025 - len(data))
                if not part: break
                data += part
            try:
                request = json.loads(data)
                assert set(request) == {'version', 'action', 'endpoint', 'project'}
                assert request['version'] == 1 and request['endpoint'] is None
                assert (request['action'] == 'preflight' and request['project'] is None) or (request['action'] == 'admit' and request['project'] == project)
                response = {'version': 1, 'ok': request['action'] == 'preflight',
                            'action': request['action'], 'endpoint': None,
                            'memoryBytes': 536870912, 'cpuQuotaUsec': 50000,
                            'cpuPeriodUsec': 100000, 'maxPids': 128,
                            'parentVerified': True, 'processBound': False}
                connection.sendall((json.dumps(response) + '\n').encode())
            except (ValueError, AssertionError, OSError):
                pass


worker = threading.Thread(target=serve, daemon=True); worker.start()
try:
    print('owned-capacity-refusal-ready', flush=True)
    sys.stdin.read()
finally:
    done.set(); worker.join(timeout=3); server.close()
    path.unlink(missing_ok=True)
    if root.exists(): root.rmdir()
