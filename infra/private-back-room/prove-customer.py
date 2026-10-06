"""Run inside the approved Briven host; print only non-secret acceptance results."""
import base64, hashlib, hmac, json, pathlib, subprocess, sys, time, urllib.parse

ENGINE = 'compose-calculate-online-transmitter-xubhp5-engine-1'
CATALOG = 'compose-calculate-online-transmitter-xubhp5-catalog-1'
ORG = 'org_proxyproof_20261006'
PSQL = '/usr/local/v17/bin/psql'
CA = '/etc/briven/proxy-ca/ca.crt'
private = []

def run(args, data=None, timeout=150):
    return subprocess.run(args, input=data, capture_output=True, text=True, timeout=timeout)

def check(condition, name):
    if not condition:
        raise RuntimeError(name)
    print(json.dumps({'check': name, 'result': 'pass'}), flush=True)

def enc(value):
    return base64.urlsafe_b64encode(json.dumps(value, separators=(',', ':')).encode()).rstrip(b'=').decode()

raw = run(['docker', 'inspect', ENGINE])
assert raw.returncode == 0, 'Private engine missing'
container = json.loads(raw.stdout)[0]
env = dict(x.split('=', 1) for x in container['Config']['Env'] if '=' in x)
private += [env.get('BRIVEN_CONTROL_IDENTITY_SECRET', ''), env.get('BRIVEN_PROXY_CONTROL_SECRET', '')]

def token(role='owner', org=ORG, expired=False, **scope):
    now = int(time.time()) - (120 if expired else 0)
    message = enc({'alg': 'HS256', 'typ': 'JWT'}) + '.' + enc({'iss': 'briven-api', 'aud': 'briven-control', 'sub': 'synthetic-proxy-proof', 'org_id': org, 'role': role, 'iat': now, 'exp': now + 60, **scope})
    signature = base64.urlsafe_b64encode(hmac.new(env['BRIVEN_CONTROL_IDENTITY_SECRET'].encode(), message.encode(), hashlib.sha256).digest()).rstrip(b'=').decode()
    value = message + '.' + signature
    private.append(value)
    return value

def api(path, method='GET', body=None, credential=None):
    config = ['url = ' + json.dumps('http://127.0.0.1:8787' + path), 'request = ' + json.dumps(method), 'silent', 'show-error', 'write-out = "\\n%{http_code}"']
    if credential is not None:
        config += ['header = ' + json.dumps('Authorization: Bearer ' + credential)]
    if body is not None:
        config += ['header = "Content-Type: application/json"', 'data = ' + json.dumps(json.dumps(body))]
    response = run(['docker', 'exec', '-i', ENGINE, 'curl', '--config', '-'], '\n'.join(config) + '\n')
    if response.returncode:
        raise RuntimeError('Private API request failed')
    payload, status = response.stdout.rsplit('\n', 1)
    try:
        decoded = json.loads(payload) if payload.strip() else None
    except json.JSONDecodeError:
        decoded = payload
    return int(status), decoded

def uri(project):
    status, response = api('/v1/projects/' + project['id'] + '/branches/main/connection', credential=token())
    if status != 200:
        raise RuntimeError('Connection issuance HTTP ' + str(status) + ': ' + str(response))
    if not (response.get('environment') == 'customer' and (response.get('expires_at') or response.get('expiresAt'))):
        raise RuntimeError('Credential metadata invalid: fields=' + ','.join(response.keys()) + ', environment=' + str(response.get('environment')))
    check(status == 200, 'scoped credential issued for ' + project['name'])
    value = response['uri']
    u = urllib.parse.urlsplit(value)
    private.extend([value, u.password])
    check(u.username == 'briven_' + project['id'] and u.hostname == 'briven-customer-db' and urllib.parse.parse_qs(u.query).get('options') == ['endpoint=ep-' + project['id']] and 'cloud_admin' not in value, 'credential binds the project endpoint')
    return value

def sql(value, query, expected=True, mode='verify-full', password=None, hostname=None):
    u = urllib.parse.urlsplit(value)
    passwd = u.password if password is None else password
    q = dict(urllib.parse.parse_qsl(u.query))
    q.update({'sslmode': mode, 'sslrootcert': CA, 'connect_timeout': '10'})
    target = urllib.parse.urlunsplit((u.scheme, u.username + '@' + (hostname or u.hostname) + ':' + str(u.port), u.path, urllib.parse.urlencode(q), ''))
    result = run(['docker', 'exec', '-i', '-e', 'LD_LIBRARY_PATH=/usr/local/v17/lib', ENGINE, '/bin/sh', '-c', 'IFS= read -r PGPASSWORD; export PGPASSWORD; exec "$@"', 'proof', PSQL, '-X', '-At', '-v', 'ON_ERROR_STOP=1', '-d', target, '-c', query], passwd + '\n', timeout=120)
    if expected and result.returncode:
        error = result.stderr
        for secret in private:
            if secret:
                error = error.replace(secret, '[redacted]')
        raise RuntimeError('Customer SQL failed: ' + error[:1200])
    return result

def admin(project, query):
    endpoint = '/var/lib/briven/endpoints/ep-' + project['id'] + '/endpoint.json'
    result = run(['docker', 'exec', ENGINE, 'cat', endpoint])
    if result.returncode:
        raise RuntimeError('Endpoint configuration missing')
    config = json.loads(result.stdout)
    return run(['docker', 'exec', '-e', 'LD_LIBRARY_PATH=/usr/local/v17/lib', ENGINE, PSQL, '-X', '-At', '-v', 'ON_ERROR_STOP=1', '-h', '127.0.0.1', '-p', str(config['pg_port']), '-U', 'cloud_admin', '-d', 'postgres', '-c', query])

def recovery():
    status, projects = api('/v1/projects', credential=token())
    assert status == 200
    a = next(p for p in projects if p['name'] == 'proxy-vector-test')
    name = 'proxy-restore-' + str(int(time.time()))
    status, dest = api('/v1/projects', 'POST', {'name': name}, token())
    check(status == 201 and dest['state'] == 'ready', 'isolated restore tenant ready')
    source_uri, dest_uri = uri(a), uri(dest)
    u = urllib.parse.urlsplit(source_uri)
    query = dict(urllib.parse.parse_qsl(u.query))
    query.update({'sslmode': 'verify-full', 'sslrootcert': CA, 'connect_timeout': '10'})
    target = urllib.parse.urlunsplit((u.scheme, u.username + '@' + u.hostname + ':' + str(u.port), u.path, urllib.parse.urlencode(query), ''))
    dump = run(['docker', 'exec', '-i', '-e', 'LD_LIBRARY_PATH=/usr/local/v17/lib', ENGINE, '/bin/sh', '-c', 'IFS= read -r PGPASSWORD; export PGPASSWORD; exec "$@"', 'proof', '/usr/local/v17/bin/pg_dump', '-d', target, '--no-owner', '--no-acl', '--table=public.briven_proxy_proof'], u.password + '\n')
    check(dump.returncode == 0 and 'CREATE TABLE' in dump.stdout, 'customer table backup through scoped TLS connection')
    folder = pathlib.Path('/var/backups/briven-engine-20261006')
    folder.mkdir(mode=0o700, exist_ok=True)
    backup = folder / ('customer-' + dest['id'] + '.sql')
    backup.write_text(dump.stdout)
    backup.chmod(0o600)
    d = urllib.parse.urlsplit(dest_uri)
    query = dict(urllib.parse.parse_qsl(d.query))
    query.update({'sslmode': 'verify-full', 'sslrootcert': CA, 'connect_timeout': '10'})
    target = urllib.parse.urlunsplit((d.scheme, d.username + '@' + d.hostname + ':' + str(d.port), d.path, urllib.parse.urlencode(query), ''))
    restored = run(['docker', 'exec', '-i', '-e', 'LD_LIBRARY_PATH=/usr/local/v17/lib', ENGINE, '/bin/sh', '-c', 'IFS= read -r PGPASSWORD; export PGPASSWORD; exec "$@"', 'proof', PSQL, '-X', '-v', 'ON_ERROR_STOP=1', '-d', target], d.password + '\n' + dump.stdout)
    check(restored.returncode == 0, 'customer backup restored into separate engine tenant')
    check(sql(dest_uri, "SELECT note FROM briven_proxy_proof ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1").stdout.strip() == 'persisted-proxy-row', 'restored rows and vector search match')
    check(sql(dest_uri, "UPDATE briven_proxy_proof SET note='restore-only' WHERE id=1; SELECT note FROM briven_proxy_proof WHERE id=1").stdout.strip().endswith('restore-only'), 'restored tenant accepts independent writes')
    check(sql(source_uri, 'SELECT note FROM briven_proxy_proof WHERE id=1').stdout.strip() == 'persisted-proxy-row', 'restore leaves source tenant unchanged')
    dumped_catalog = run(['docker', 'exec', CATALOG, 'pg_dump', '-U', 'briven_catalog', '-d', 'briven_control', '--no-owner', '--no-acl'])
    check(dumped_catalog.returncode == 0 and 'briven_control.projects' in dumped_catalog.stdout, 'engine catalog backup')
    catalog_backup = folder / ('catalog-' + dest['id'] + '.sql')
    catalog_backup.write_text(dumped_catalog.stdout)
    catalog_backup.chmod(0o600)
    dbname = 'briven_control_proof_' + dest['id'][:12]
    check(run(['docker', 'exec', CATALOG, 'createdb', '-U', 'briven_catalog', dbname]).returncode == 0, 'isolated catalog restore database created')
    catalog_restore = run(['docker', 'exec', '-i', CATALOG, 'psql', '-X', '-U', 'briven_catalog', '-d', dbname, '-v', 'ON_ERROR_STOP=1'], dumped_catalog.stdout)
    check(catalog_restore.returncode == 0, 'catalog backup restored')
    count = run(['docker', 'exec', CATALOG, 'psql', '-X', '-At', '-U', 'briven_catalog', '-d', dbname, '-c', "SELECT count(*) FROM briven_control.projects WHERE organization_id='" + ORG + "'"])
    check(count.returncode == 0 and count.stdout.strip() == str(len(projects) + 1), 'restored catalog contains all synthetic project mappings')
    rollback = run(['docker', 'exec', CATALOG, 'psql', '-X', '-U', 'briven_catalog', '-d', dbname, '-v', 'ON_ERROR_STOP=1', '-c', 'BEGIN; CREATE TABLE rollback_probe(id int); SELECT 1/0; COMMIT;'])
    check(rollback.returncode != 0, 'failed migration aborts on catalog copy')
    absent = run(['docker', 'exec', CATALOG, 'psql', '-X', '-At', '-U', 'briven_catalog', '-d', dbname, '-c', "SELECT to_regclass('public.rollback_probe') IS NULL"])
    check(absent.stdout.strip() == 't', 'failed migration leaves no partial table')
    check(run(['docker', 'exec', CATALOG, 'dropdb', '-U', 'briven_catalog', dbname]).returncode == 0, 'temporary catalog copy cleaned up')
    print(json.dumps({'phase': 'backup-restore', 'result': 'pass', 'customerBackup': str(backup), 'catalogBackup': str(catalog_backup)}), flush=True)

def persisted():
    status, projects = api('/v1/projects', credential=token())
    check(status == 200, 'catalog available after full engine restart')
    a = next(p for p in projects if p['name'] == 'proxy-vector-test')
    value = uri(a)
    check(sql(value, "SELECT note FROM briven_proxy_proof WHERE id=1").stdout.strip() == 'persisted-proxy-row', 'customer row survives full engine restart')
    check(sql(value, "SELECT note FROM briven_proxy_proof ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1").stdout.strip() == 'persisted-proxy-row', 'pgvector search survives full engine restart')
    processes = run(['docker', 'top', ENGINE, '-eo', 'pid,comm']).stdout
    check(sum(line.split()[-1] == 'safekeeper' for line in processes.splitlines()[1:]) == 3, 'three durable safekeepers running after restart')
    durable = run(['docker', 'exec', '-e', 'LD_LIBRARY_PATH=/usr/local/v16/lib', ENGINE, '/usr/local/v16/bin/psql', '-X', '-At', '-h', '127.0.0.1', '-p', '1235', '-U', 'neon', '-d', 'storage_controller', '-c', 'SHOW fsync'])
    check(durable.stdout.strip() == 'on', 'controller durable writes preserved after restart')
    print(json.dumps({'phase': 'engine-restart', 'result': 'pass'}), flush=True)

def services():
    status, projects = api('/v1/projects', credential=token())
    check(status == 200, 'service proof catalog available')
    a = next(p for p in projects if p['name'] == 'proxy-vector-test')
    b = next(p for p in projects if p['name'] == 'proxy-isolation-test')
    base = '/v1/projects/' + a['id'] + '/branches/main/'
    assertion = token(role='admin', service_project=a['id'], service_kind='runtime')
    for path in ['/v1/projects', base + 'connection', '/v1/projects/' + a['id']]:
        check(api(path, credential=assertion)[0] == 403, 'scoped service cannot use customer route ' + path)
    check(api(base + 'service-connection', credential=token())[0] == 403, 'customer assertion cannot issue service credentials')
    check(api(base + 'service-connection', credential=token(service_project=a['id'], service_kind='runtime', role='viewer'))[0] == 403, 'viewer cannot issue service credentials')
    check(api('/v1/projects/' + b['id'] + '/branches/main/service-connection', credential=assertion)[0] == 403, 'service assertion cannot cross tenants')
    leases = {}
    for kind in ['platform', 'runtime']:
        status, lease = api(base + 'service-connection', credential=token(role='admin', service_project=a['id'], service_kind=kind))
        check(status == 200 and lease['environment'] == 'service' and lease['expiresAt'], kind + ' service lease issued')
        value = lease['uri']
        u = urllib.parse.urlsplit(value)
        private.extend([value, u.password])
        check(u.username == 'briven_' + a['id'] + '_' + kind, kind + ' role exact binding')
        check(sql(value, 'SELECT current_user').stdout.strip() == u.username, kind + ' verified TLS login')
        leases[kind] = value
    customer = uri(a)
    for kind, value in [('customer', customer), ('runtime', leases['runtime'])]:
        check(sql(value, 'SELECT * FROM public._briven_meta', False).returncode != 0, kind + ' cannot read platform metadata')
        check(sql(value, 'SET ROLE briven_' + a['id'] + '_platform; SELECT 1', False).returncode != 0, kind + ' cannot assume platform role')
    check(sql(leases['platform'], "INSERT INTO public._briven_meta VALUES ('proxy_service_proof', '{\"ok\":true}') ON CONFLICT (key) DO UPDATE SET value=excluded.value; SELECT value->>'ok' FROM public._briven_meta WHERE key='proxy_service_proof'").stdout.strip().endswith('true'), 'platform metadata write/read')
    check(sql(leases['runtime'], 'SET ROLE briven_' + a['id'] + "; CREATE TABLE IF NOT EXISTS public.briven_service_proof(id integer PRIMARY KEY, embedding vector(3)); INSERT INTO briven_service_proof VALUES (1,'[1,0,0]') ON CONFLICT DO NOTHING; SELECT id FROM briven_service_proof ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1").stdout.strip().endswith('1'), 'runtime application role vector write/search')
    check(sql(customer, 'ALTER TABLE public.briven_service_proof ADD COLUMN IF NOT EXISTS note text; SELECT count(*) FROM briven_service_proof').stdout.strip().endswith('1'), 'customer owns runtime-created application tables')
    replacement = uri(a)
    check(sql(customer, 'SELECT 1', False).returncode != 0, 'customer credential rotated independently')
    for kind, value in leases.items():
        check(sql(value, 'SELECT count(*) FROM public.briven_service_proof').stdout.strip() == '1', kind + ' unaffected by customer password rotation')
        u = urllib.parse.urlsplit(value)
        q = dict(urllib.parse.parse_qsl(u.query)); q['options'] = 'endpoint=ep-' + b['id']
        cross = urllib.parse.urlunsplit((u.scheme, u.netloc, u.path, urllib.parse.urlencode(q), ''))
        check(sql(cross, 'SELECT 1', False).returncode != 0, kind + ' proxy cannot open another tenant')
    print(json.dumps({'phase': 'service-roles', 'result': 'pass'}), flush=True)

def main():
    check(not container['HostConfig'].get('PortBindings'), 'no public engine ports')
    status, _ = api('/healthz')
    check(status == 200, 'private control API health')
    check(api('/v1/projects')[0] == 401, 'missing identity rejected')
    check(api('/v1/projects', credential=token(expired=True))[0] == 401, 'expired identity rejected')
    check(api('/v1/projects', 'POST', {'name': 'viewer-test'}, token('viewer'))[0] == 403, 'viewer creation rejected')
    check(api('/proxy/wake_compute?endpointish=ep-unknown', credential=token())[0] == 401, 'website identity rejected by proxy callbacks')
    projects = []
    status, listed = api('/v1/projects', credential=token())
    assert status == 200
    if isinstance(listed, dict):
        listed = listed['projects']
    for name in ['proxy-vector-test', 'proxy-isolation-test']:
        project = next((p for p in listed if p['name'] == name), None)
        if project is None:
            status, project = api('/v1/projects', 'POST', {'name': name}, token())
            check(status in (200, 201) and project.get('state') == 'ready', 'synthetic project ready: ' + name)
        else:
            check(project.get('state') == 'ready', 'existing synthetic project ready: ' + name)
        projects.append(project)
    a, b = projects
    check(api('/v1/projects', 'POST', {'name': a['name']}, token())[0] == 409, 'duplicate project creation rejected')
    check(api('/v1/projects/' + a['id'], credential=token(org='org_other_proxyproof'))[0] == 404, 'organization isolation')
    check(api('/v1/projects/' + a['id'] + '/branches/main/connection', credential=token('viewer'))[0] == 403, 'viewer write credential rejected')
    a_uri, b_uri = uri(a), uri(b)
    check(sql(a_uri, 'SELECT current_user').stdout.strip() == 'briven_' + a['id'], 'hostname verified TLS customer login')
    check(sql(a_uri, 'SELECT 1', False, password='invalid-proof-password').returncode != 0, 'wrong password rejected')
    check(sql(a_uri, 'SELECT 1', False, mode='disable').returncode != 0, 'plaintext connection rejected')
    bad = urllib.parse.urlsplit(a_uri)
    check(sql(urllib.parse.urlunsplit((bad.scheme, 'cloud_admin:' + bad.password + '@' + bad.hostname + ':' + str(bad.port), bad.path, bad.query, '')), 'SELECT 1', False).returncode != 0, 'administrator login rejected')
    other_query = dict(urllib.parse.parse_qsl(bad.query))
    other_query['options'] = 'endpoint=ep-' + b['id']
    crossed = urllib.parse.urlunsplit((bad.scheme, bad.netloc, bad.path, urllib.parse.urlencode(other_query), ''))
    check(sql(crossed, 'SELECT 1', False).returncode != 0, 'credential cannot open another tenant')
    query = "CREATE TABLE IF NOT EXISTS public.briven_proxy_proof(id integer PRIMARY KEY, note text NOT NULL, embedding vector(3)); INSERT INTO public.briven_proxy_proof VALUES (1,'persisted-proxy-row','[1,0,0]'),(2,'second-vector','[0,1,0]') ON CONFLICT (id) DO NOTHING; SELECT note FROM public.briven_proxy_proof ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1;"
    check(sql(a_uri, query).stdout.strip().endswith('persisted-proxy-row'), 'customer write and pgvector similarity search')
    check(sql(b_uri, "SELECT count(*) FROM information_schema.tables WHERE table_name='briven_proxy_proof'").stdout.strip() == '0', 'customer rows isolated')
    check(run(['docker', 'exec', ENGINE, 'briven_local', 'endpoint', 'stop', 'ep-' + a['id']]).returncode == 0, 'synthetic compute stopped')
    check(sql(a_uri, "SELECT note FROM briven_proxy_proof WHERE id=1").stdout.strip() == 'persisted-proxy-row', 'proxy resumes compute and preserves row')
    check(admin(a, "ALTER ROLE briven_" + a['id'] + " VALID UNTIL '2000-01-01'").returncode == 0, 'synthetic credential expired')
    check(sql(a_uri, 'SELECT 1', False).returncode != 0, 'expired database credential rejected')
    fresh = uri(a)
    check(sql(fresh, 'SELECT count(*) FROM briven_proxy_proof').stdout.strip() == '2', 'new credential restores scoped access')
    newer = uri(a)
    check(sql(fresh, 'SELECT 1', False).returncode != 0, 'credential rotation revokes previous password')
    check(sql(newer, 'SELECT count(*) FROM briven_proxy_proof').stdout.strip() == '2', 'replacement credential works')
    print(json.dumps({'phase': 'customer-proxy', 'result': 'pass', 'projects': [{'id': p['id'], 'name': p['name']} for p in projects]}), flush=True)

try:
    phase = sys.argv[1] if len(sys.argv) > 1 else 'customer'
    {'customer': main, 'recovery': recovery, 'persisted': persisted, 'services': services}[phase]()
except Exception as error:
    message = str(error)
    for secret in private:
        if secret:
            message = message.replace(secret, '[redacted]')
    print(json.dumps({'result': 'fail', 'error': message[:1500]}), flush=True)
    sys.exit(1)
