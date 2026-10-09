#!/usr/bin/env python3
"""Fixed Briven resource bridge; no caller-selected command, PID or limits.

Only the exact private engine PID namespace and UID may request resources.
systemd remains the sole cgroup writer. A prepare request moves its authenticated
launcher, before exec/fork, into a bounded transient scope.
"""
import json
import os
from pathlib import Path
import re
import select
import socket
import struct
import subprocess
import time

SOCKET = "/run/briven-compute-control/control.sock"
ENGINE = "compose-calculate-online-transmitter-xubhp5-engine-1"
COMPOSE_PROJECT = "compose-calculate-online-transmitter-xubhp5"
SLICE = "brivencompute.slice"
CGROUP = Path("/sys/fs/cgroup") / SLICE
ENDPOINT = re.compile(r"ep-([0-9a-f]{32})(?:-[a-z0-9]+(?:-[a-z0-9]+)*)?", re.ASCII)
PROJECT = re.compile(r"[0-9a-f]{32}", re.ASCII)
LIMITS = {"memory.max": "536870912", "memory.swap.max": "0",
          "cpu.max": "50000 100000", "pids.max": "128", "memory.oom.group": "1"}
PARENT_LIMITS = {"memory.max": "8589934592", "memory.swap.max": "0",
                 "cpu.max": "300000 100000", "pids.max": "1024"}


def require(condition):
    if not condition:
        raise ValueError("resource request refused")


def endpoint_project(endpoint):
    require(isinstance(endpoint, str) and len(endpoint) <= 96)
    match = ENDPOINT.fullmatch(endpoint)
    require(match is not None)
    return match.group(1)


def command(args):
    # These arguments are constructed below from constants or strict identifiers.
    # Capture, never log raw subprocess errors, Docker configuration or argv.
    result = subprocess.run(args, capture_output=True, timeout=3, check=False)
    require(result.returncode == 0 and len(result.stdout) <= 4096)
    return result.stdout.decode("utf-8").strip()


def read(path):
    require(not path.is_symlink())
    data = path.read_text()
    require(len(data) <= 16384)
    return data.strip()


def limits(path, expected):
    require(path.is_dir() and not path.is_symlink())
    for key, value in expected.items():
        require(read(path / key) == value)


def parent():
    require(read(Path("/sys/fs/cgroup/cgroup.controllers")) != "")
    limits(CGROUP, PARENT_LIMITS)


def populated(path):
    fields = dict(line.split() for line in read(path / "cgroup.events").splitlines())
    require(fields.get("populated") in ("0", "1"))
    return fields["populated"] == "1"


def active_endpoints():
    result = []
    for path in CGROUP.iterdir():
        name = path.name
        if not name.startswith("briven-compute-"):
            continue
        require(name.endswith(".scope"))
        endpoint = name[len("briven-compute-"):-len(".scope")]
        endpoint_project(endpoint)
        if populated(path):
            limits(path, LIMITS)
            result.append(endpoint)
    require(len(result) <= 4)
    return result


def admission(project, active):
    require(PROJECT.fullmatch(project) is not None)
    require(len(active) < 4)
    require(sum(endpoint_project(value) == project for value in active) < 2)


def start_identity(pid):
    # comm may contain spaces/parentheses; starttime is field 22 after final ')'.
    tail = Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()
    require(tail[0] not in ("Z", "X"))
    return tail[19]


def authenticate(connection, request):
    pid, uid, _gid = struct.unpack("3i", connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
    require(uid == 1000 and pid > 1)
    identity = start_identity(pid)
    # Narrow inspect output deliberately excludes environment variables/secrets.
    inspected = json.loads(command([
        "/usr/bin/docker", "inspect", "--format",
        '{"pid":{{.State.Pid}},"running":{{.State.Running}},'
        '"user":{{json .Config.User}},"project":{{json (index .Config.Labels "com.docker.compose.project")}},'
        '"service":{{json (index .Config.Labels "com.docker.compose.service")}}}', ENGINE]))
    require(inspected["running"] is True and inspected["pid"] > 1)
    require(inspected["project"] == COMPOSE_PROJECT and inspected["service"] == "engine")
    require(inspected["user"] in ("neon", "1000", "1000:1000", "neon:neon"))
    require(os.readlink(f"/proc/{pid}/ns/pid") == os.readlink(f'/proc/{inspected["pid"]}/ns/pid'))
    executable = os.readlink(f"/proc/{pid}/exe")
    allowed = {"/usr/local/bin/briven_control_api"}
    if request["action"] == "prepare":
        allowed = {"/usr/local/bin/briven_compute_launch"}
    elif request["action"] == "admit":
        allowed.update(("/usr/local/bin/briven_local", "/usr/local/bin/neon_local"))
    require(executable in allowed)
    if request["action"] == "prepare":
        argv = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
        endpoint = request["endpoint"].encode("ascii")
        require(len(argv) > 2 and argv[1] == endpoint)
        if argv[2] == b"--resource-probe":
            require(len(argv) == 5 and argv[3] in (b"cpu", b"memory", b"pids") and argv[4] == b"")
            environment = Path(f"/proc/{pid}/environ").read_bytes().split(b"\0")
            require(b"BRIVEN_RESOURCE_PROBE_ENABLED=true" in environment)
            require(not Path(f"/proc/{pid}/root/var/lib/briven/endpoints/{request['endpoint']}").exists())
        else:
            require(argv.count(b"--compute-id") == 1)
            index = argv.index(b"--compute-id")
            require(index + 1 < len(argv) and argv[index + 1] == endpoint)
            require(argv.count(b"--pgdata") == 1)
            index = argv.index(b"--pgdata")
            require(index + 1 < len(argv) and argv[index + 1] == b"/var/lib/briven/endpoints/" + endpoint + b"/pgdata")
    require(start_identity(pid) == identity)
    return pid, identity


def validate_request(value):
    require(isinstance(value, dict) and set(value) == {"version", "action", "endpoint", "project"})
    require(type(value["version"]) is int and value["version"] == 1)
    action, endpoint, project = value["action"], value["endpoint"], value["project"]
    require(action in ("preflight", "admit", "prepare", "verify"))
    if action == "preflight":
        require(endpoint is None and project is None)
    elif action == "admit":
        require(endpoint is None and isinstance(project, str) and PROJECT.fullmatch(project) is not None)
    else:
        require(endpoint_project(endpoint) == project)


def unique_fields(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value)
        value[key] = item
    return value


def prepare(endpoint, pid, identity):
    unit = f"briven-compute-{endpoint}.scope"
    path = CGROUP / unit
    require(not path.exists() or not populated(path))
    # Reset only this exact empty scope's failure state. Never stop/kill a unit.
    subprocess.run(["/usr/bin/systemctl", "reset-failed", unit], capture_output=True, timeout=3, check=False)
    require(start_identity(pid) == identity)
    pidfd = os.pidfd_open(pid)
    try:
        require(not select.select([pidfd], [], [], 0)[0])
        command(["/usr/bin/busctl", "call", "org.freedesktop.systemd1", "/org/freedesktop/systemd1",
                 "org.freedesktop.systemd1.Manager", "StartTransientUnit", "ssa(sv)a(sa(sv))",
                 unit, "fail", "11",
                 "PIDs", "au", "1", str(pid),
                 "Slice", "s", SLICE,
                 "CPUAccounting", "b", "true", "MemoryAccounting", "b", "true", "TasksAccounting", "b", "true",
                 "CPUQuotaPerSecUSec", "t", "500000", "CPUQuotaPeriodUSec", "t", "100000",
                 "MemoryMax", "t", "536870912", "MemorySwapMax", "t", "0", "TasksMax", "t", "128",
                 "OOMPolicy", "s", "kill", "0"])
        require(not select.select([pidfd], [], [], 0)[0] and start_identity(pid) == identity)
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            if path.exists() and str(pid) in read(path / "cgroup.procs").splitlines():
                break
            time.sleep(0.025)
        limits(path, LIMITS)
        require(str(pid) in read(path / "cgroup.procs").splitlines())
    finally:
        os.close(pidfd)


def serve_request(connection):
    connection.settimeout(4)
    raw = bytearray()
    while not raw.endswith(b"\n") and len(raw) <= 1024:
        part = connection.recv(min(1025 - len(raw), 1025))
        require(bool(part))
        raw.extend(part)
    require(len(raw) <= 1024 and raw.endswith(b"\n") and raw.count(b"\n") == 1)
    request = json.loads(raw, object_pairs_hook=unique_fields)
    validate_request(request)
    pid, identity = authenticate(connection, request)
    parent()
    active = active_endpoints()
    action, endpoint, project = request["action"], request["endpoint"], request["project"]
    if action in ("admit", "prepare"):
        admission(project, active)
    if action == "prepare":
        require(endpoint not in active)
        prepare(endpoint, pid, identity)
    elif action == "verify":
        require(endpoint in active)
        limits(CGROUP / f"briven-compute-{endpoint}.scope", LIMITS)
    require(start_identity(pid) == identity)
    response = {"version": 1, "ok": True, "action": action, "endpoint": endpoint,
                "memoryBytes": 536870912, "cpuQuotaUsec": 50000, "cpuPeriodUsec": 100000,
                "maxPids": 128, "parentVerified": True, "processBound": action == "prepare"}
    connection.sendall(json.dumps(response, separators=(",", ":")).encode("ascii") + b"\n")


def main():
    require(os.geteuid() == 0)
    parent()
    directory = Path(SOCKET).parent
    require(directory.is_dir() and not directory.is_symlink() and directory.stat().st_uid == 0)
    # The container's numeric GID need not have a host NSS group entry.
    # RuntimeDirectory is created as root:root, then this fixed directory is
    # assigned the kernel numeric GID. No new host account/group is created.
    os.chown(directory, 0, 1000)
    os.chmod(directory, 0o750)
    # RuntimeDirectory owns this fixed path; never replace an existing socket.
    require(not os.path.lexists(SOCKET))
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(SOCKET)
    os.chown(SOCKET, 0, 1000)
    os.chmod(SOCKET, 0o660)
    server.listen(8)
    try:
        while True:
            connection, _ = server.accept()
            with connection:
                try:
                    serve_request(connection)
                except (ValueError, KeyError, TypeError, IndexError, UnicodeError, OSError, subprocess.SubprocessError):
                    # No raw request, subprocess output, process args or secrets.
                    try:
                        connection.sendall(b'{"ok":false}\n')
                    except OSError:
                        pass
    finally:
        server.close()
        Path(SOCKET).unlink(missing_ok=True)


if __name__ == "__main__":
    main()
