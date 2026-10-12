#!/usr/bin/env python3
"""Read only the fixed aggregate cgroup; atomically publish machine metrics."""
import os
from pathlib import Path
import re
import tempfile
import time

CGROUP = Path('/sys/fs/cgroup/brivencompute.slice')
OUTPUT = Path('/run/briven-compute-metrics/compute.prom')
BOUNDS = {'memory.max': 8589934592, 'memory.swap.max': 0, 'pids.max': 1024}


def read(path):
    if any(part.is_symlink() for part in [path, *path.parents]):
        raise ValueError('Symlink refused')
    with path.open() as source:
        value = source.read(4097)
    if len(value) > 4096:
        raise ValueError('Oversized accounting file')
    return value.strip()


def number(raw):
    if not re.fullmatch(r'[0-9]{1,20}', raw, re.ASCII):
        raise ValueError('Invalid accounting value')
    return int(raw)


def collect(group=CGROUP, now=None):
    # Parameters are for filesystem tests only; the installed CLI has no arguments.
    for name, expected in BOUNDS.items():
        if number(read(group / name)) != expected:
            raise ValueError('Unexpected resource bound')
    if read(group / 'cpu.max') != '300000 100000':
        raise ValueError('Unexpected CPU bound')
    rows = [line.split() for line in read(group / 'cpu.stat').splitlines()]
    if any(len(row) != 2 for row in rows) or len({row[0] for row in rows}) != len(rows):
        raise ValueError('Malformed CPU accounting')
    cpu = {key: number(value) for key, value in rows}
    memory = number(read(group / 'memory.current'))
    tasks = number(read(group / 'pids.current'))
    # Transient over-limit kernel readings must stay unknown, never be clamped.
    if memory > BOUNDS['memory.max'] or tasks > BOUNDS['pids.max']:
        raise ValueError('Out-of-range accounting')
    return {
        'collection_success': 1,
        'sampled_timestamp_seconds': time.time() if now is None else now,
        'cpu_usage_seconds_total': cpu['usage_usec'] / 1_000_000,
        'cpu_limit_cores': 3,
        'memory_current_bytes': memory,
        'memory_limit_bytes': BOUNDS['memory.max'],
        'tasks_current': tasks,
        'tasks_limit': BOUNDS['pids.max'],
    }


def publish(output=OUTPUT, group=CGROUP, now=None):
    try:
        values = collect(group, now)
    except (OSError, ValueError, KeyError):
        # Replace previous success. A stopped producer is detected by its old sample time.
        values = {'collection_success': 0}
    text = ''.join(
        f'briven_compute_{name}{{group="brivencompute.slice"}} {value}\n'
        for name, value in values.items()
    )
    if output.is_symlink() or any(part.is_symlink() for part in output.parents):
        raise ValueError('Symlink refused')
    fd, temporary = tempfile.mkstemp(prefix='.compute-', dir=output.parent)
    try:
        with os.fdopen(fd, 'w') as dest:
            os.fchmod(dest.fileno(), 0o644)
            dest.write(text)
            dest.flush()
            os.fsync(dest.fileno())
        os.replace(temporary, output)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    return values['collection_success'] == 1


if __name__ == '__main__':
    import sys
    if len(sys.argv) != 1:
        raise SystemExit('No arguments accepted')
    raise SystemExit(0 if publish() else 1)
