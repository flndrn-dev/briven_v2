/** Run from outside KVM2; ephemeral customer leases arrive only on stdin. */
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { createConnection } from 'node:net';
const { default: pg } = await import(pathToFileURL(process.env.BRIVEN_PROOF_PG_MODULE).href);
const input = JSON.parse(readFileSync(0, 'utf8'));
const check = (condition, name) => {
  if (!condition) throw new Error(name);
  console.log(JSON.stringify({ check: name, result: 'pass' }));
};
function uri(value) {
  const url = new URL(value);
  url.hostname = 'briven.tech';
  // Explicit TLS config must not be overwritten by URI parameters.
  for (const key of ['sslmode', 'sslrootcert', 'sslcert', 'sslkey', 'uselibpqcompat']) url.searchParams.delete(key);
  return url;
}
async function connect(url, ssl = { rejectUnauthorized: true }) {
  const client = new pg.Client({ connectionString: String(url), ssl, connectionTimeoutMillis: 10000 });
  client.on('error', () => {});
  try { await client.connect(); return client; } catch (error) { await client.end().catch(() => {}); throw error; }
}
async function rejected(url, ssl) {
  let client;
  try { client = await connect(url, ssl); return false; } catch { return true; }
  finally { await client?.end().catch(() => {}); }
}
async function portClosed(port) {
  return new Promise(resolve => {
    const socket = createConnection({ host: '187.77.183.190', port });
    const finish = value => { socket.destroy(); resolve(value); };
    socket.setTimeout(2000);
    socket.once('connect', () => finish(false));
    socket.once('timeout', () => finish(true));
    socket.once('error', () => finish(true));
  });
}
let client;
try {
  const current = uri(input.current.uri);
  client = await connect(current);
  check(client.connection.stream.encrypted && client.connection.stream.authorized,
    'public PostgreSQL verifies hostname and system-trusted TLS chain');
  const rows = await client.query("SELECT note FROM briven_website_proof ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1");
  check(rows.rows[0]?.note === 'nearest', 'external customer pgvector similarity query');
  const role = await client.query('SELECT current_user AS role');
  check(role.rows[0].role === `briven_${input.current.engineProjectId}`, 'external connection is customer-scoped');
  let metadataDenied = false;
  try { await client.query('SELECT * FROM _briven_meta'); } catch (error) { metadataDenied = error.code === '42501'; }
  check(metadataDenied, 'external customer cannot read platform metadata');
  check(await rejected(uri(input.revoked.uri)), 'rotated customer password is rejected externally');
  const wrongPassword = new URL(current); wrongPassword.password = '0'.repeat(64);
  check(await rejected(wrongPassword), 'public proxy rejects wrong password');
  const wrongTenant = new URL(current); wrongTenant.searchParams.set('options', 'endpoint=ep-af68ba39fbd67f468f5474d1e4a0812c');
  check(await rejected(wrongTenant), 'public proxy rejects cross-tenant routing');
  const admin = new URL(current); admin.username = 'cloud_admin';
  check(await rejected(admin), 'public proxy rejects administrator role');
  check(await rejected(current, false), 'public proxy rejects plaintext');
  const wrongHostname = new URL(current); wrongHostname.hostname = '187.77.183.190';
  check(await rejected(wrongHostname), 'public TLS rejects hostname mismatch');
  check((await Promise.all([8787, 7000, 7001, 55440].map(portClosed))).every(Boolean),
    'control, proxy management and administrator compute ports remain closed externally');
  console.log(JSON.stringify({ phase: 'public-postgres', result: 'pass' }));
} catch (error) {
  // Never surface driver errors containing credentials or connection URIs.
  console.log(JSON.stringify({ phase: 'public-postgres', result: 'fail', error: String(error.message).replace(/postgres(?:ql)?:\/\/[^\s"']+/g, '[redacted]').slice(0, 200) }));
  process.exitCode = 1;
} finally { await client?.end().catch(() => {}); }
