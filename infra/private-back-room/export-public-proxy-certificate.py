"""Export only Briven's existing public certificate from the host ACME store.

Run as a root-owned systemd oneshot. Keys never enter arguments or logs. An
atomic symlink switch gives the proxy one coherent certificate/key generation.
"""
import base64
import hashlib
import json
import os
import pathlib
import subprocess
import tempfile

DOMAIN = 'briven.tech'
STORE = pathlib.Path('/etc/dokploy/traefik/dynamic/acme.json')
DESTINATION = pathlib.Path('/etc/briven/public-postgres-tls')


def select_certificate(store):
    for resolver in store.values():
        for certificate in resolver.get('Certificates', []):
            domain = certificate.get('domain', {})
            if DOMAIN in [domain.get('main'), *domain.get('sans', [])]:
                return certificate
    raise RuntimeError('Briven public certificate is not available')


def openssl(arguments, data=None):
    result = subprocess.run(['openssl', *arguments], input=data, capture_output=True)
    if result.returncode:
        raise RuntimeError('Briven public certificate verification failed')
    return result.stdout


def export():
    entry = select_certificate(json.loads(STORE.read_text()))
    chain = base64.b64decode(entry['certificate'], validate=True)
    key = base64.b64decode(entry['key'], validate=True)
    openssl(['x509', '-noout', '-checkhost', DOMAIN, '-checkend', '604800'], chain)
    public = openssl(['x509', '-pubkey', '-noout'], chain)
    if public != openssl(['pkey', '-pubout'], key):
        raise RuntimeError('Briven public certificate key does not match')
    with tempfile.NamedTemporaryFile() as cert:
        cert.write(chain)
        cert.flush()
        openssl(['verify', '-verify_hostname', DOMAIN, '-CAfile', '/etc/ssl/certs/ca-certificates.crt', '-untrusted', cert.name, cert.name])
    generation = hashlib.sha256(chain).hexdigest()
    DESTINATION.mkdir(mode=0o750, parents=True, exist_ok=True)
    os.chown(DESTINATION, 0, 1000)
    current = DESTINATION / 'current'
    if current.is_symlink() and os.readlink(current) == generation:
        return
    folder = DESTINATION / generation
    folder.mkdir(mode=0o750, exist_ok=True)
    os.chown(folder, 1000, 1000)
    for filename, value, mode in [('tls.crt', chain, 0o644), ('tls.key', key, 0o600)]:
        path = folder / filename
        with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, mode), 'wb') as target:
            target.write(value)
            target.flush()
            os.fsync(target.fileno())
        os.chown(path, 1000, 1000)
        os.chmod(path, mode)
    pending = DESTINATION / 'current.next'
    if pending.is_symlink(): pending.unlink()
    pending.symlink_to(generation)
    pending.replace(current)
    print('Briven public PostgreSQL certificate updated', flush=True)


if __name__ == '__main__':
    try:
        export()
    except Exception:
        raise SystemExit('Briven public PostgreSQL certificate export failed')
