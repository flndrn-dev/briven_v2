"""Project no raw PostgreSQL log text into acceptance evidence."""
import datetime
import re

LINE = re.compile(r'(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d\.\d{3} UTC) \[(\d+)\] BRIVEN_ACCEPTANCE_SESSION app=([^\n\u200b]*?)\| (?:LOG|FATAL):\s+([^\n\u200b]*)')
CLASSES = ('customer', 'monitor', 'inspection', 'other')


def classify(role, app, expected_role):
    if role == expected_role:
        return 'customer'
    if app == 'compute_ctl:compute_monitor':
        return 'monitor'
    if app == 'briven_lifecycle_inspection':
        return 'inspection'
    return 'other'


def summarize(raw, expected_role, last_demand, fault_window=None, observed_at=None, bytes_read=None):
    assert len(raw) <= 2097152
    completed, pending = [], {}
    parsed, first_tagged, last_tagged = 0, None, None
    for match in LINE.finditer(raw):
        parsed += 1
        assert parsed <= 20000
        at = datetime.datetime.fromisoformat(match[1].replace(' UTC', '+00:00')).timestamp()
        first_tagged = at if first_tagged is None else min(first_tagged, at)
        last_tagged = at if last_tagged is None else max(last_tagged, at)
        backend, app, message = int(match[2]), match[3], match[4]
        assert 0 < backend < 2**31
        role = re.search(r'(?:^| )user=([^ ]+)', message)
        kind = classify(role[1] if role else None, app, expected_role)
        if message.startswith('connection received:'):
            pending[backend] = {'backend': backend, 'at': at, 'kind': 'other', 'stage': 'received'}
        elif message.startswith('connection authorized:'):
            previous = pending.get(backend)
            pending[backend] = {'backend': backend, 'at': previous['at'] if previous else at, 'authorizedAt': at, 'kind': kind, 'stage': 'authorized'}
        elif message.startswith('disconnection: session time:'):
            duration = re.match(r'disconnection: session time: (\d+):(\d\d):(\d\d)\.(\d{3}) ', message)
            if not duration:
                continue
            milliseconds = ((int(duration[1])*60+int(duration[2]))*60+int(duration[3]))*1000+int(duration[4])
            assert 0 <= milliseconds <= 172800000
            start = round(at-milliseconds/1000, 3)
            event = {'backend': backend, 'kind': kind, 'startedAt': start, 'endedAt': at, 'durationMs': milliseconds}
            event['overlapsInjectedPause'] = bool(fault_window and start <= fault_window[1] and at >= fault_window[0])
            completed.append(event)
            pending.pop(backend, None)
    counts = {kind: sum(event['kind'] == kind for event in completed) for kind in CLASSES}
    maximum = {kind: max((event['durationMs'] for event in completed if event['kind'] == kind), default=0) for kind in CLASSES}
    outside = {kind: max((event['durationMs'] for event in completed if event['kind'] == kind and not event['overlapsInjectedPause']), default=0) for kind in CLASSES}
    near = [event for event in completed if event['startedAt'] <= last_demand+1 and event['endedAt'] >= last_demand]
    observed_at = max(last_demand, last_tagged or 0) if observed_at is None else observed_at
    unpaired = [{'backend': event['backend'], 'kind': event['kind'], 'stage': event['stage'],
                 'startedAt': event['at'], 'authorizedAt': event.get('authorizedAt'),
                 'observedAgeMs': round(max(0, observed_at-event['at'])*1000),
                 'pairStatus': 'unpaired-log-events'} for event in pending.values()]
    unpaired.sort(key=lambda event: event['startedAt'])
    unpaired_near = [event for event in unpaired if event['startedAt'] <= last_demand+1]
    bytes_read = len(raw.encode()) if bytes_read is None else bytes_read
    coverage = {'bytesRead': bytes_read, 'byteLimit': 2097152, 'tailMayBeTruncated': bytes_read == 2097152,
                'firstTaggedAt': first_tagged, 'lastTaggedAt': last_tagged, 'observedAt': observed_at,
                'demandIntervalWithinTaggedSpan': first_tagged is not None and first_tagged <= last_demand and last_tagged >= last_demand+1}
    return {'eventsParsed': parsed, 'completed': counts, 'maxDurationMs': maximum,
            'maxDurationOutsidePauseMs': outside, 'lastDemand': last_demand,
            'nearDemandTotal': len(near), 'nearDemand': near[-32:],
            'unpairedEvents': len(pending), 'unpairedByClass': {kind: sum(event['kind'] == kind for event in unpaired) for kind in CLASSES},
            'unpaired': unpaired[:32], 'unpairedTruncated': len(unpaired)>32,
            'unpairedNearDemandTotal': len(unpaired_near), 'unpairedNearDemand': unpaired_near[:32],
            'coverage': coverage, 'latest': completed[-8:]}
