/** Execute inside the staged API image; owner credentials arrive on stdin. */
import { readFileSync } from 'node:fs';
const owner = JSON.parse(readFileSync(0, 'utf8'));
const hidden = [owner.password, process.env.BRIVEN_RUNTIME_SHARED_SECRET];
const check = (condition, name) => {
  if (!condition) throw new Error(name);
  console.log(JSON.stringify({ check: name, result: 'pass' }));
};
let cookie = '';
async function request(path, method = 'GET', body, authorized = true) {
  const res = await fetch('http://127.0.0.1:3001' + path, {
    method, headers: { origin: 'https://briven.tech', 'x-forwarded-for': '127.0.0.1', ...(cookie && authorized ? { cookie } : {}),
      ...(body === undefined ? {} : { 'content-type': 'application/json' }) },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }), signal: AbortSignal.timeout(150_000),
  });
  const data = await res.json().catch(() => null);
  if (!res.ok) console.log(JSON.stringify({ request: path, status: res.status, code: data?.code, message: data?.message }));
  return { res, data };
}
let manager;
let socket, proofKeyId, proofProjectId;
let closeDataPlane, closeDb, closeEnginePool;
async function checkAi(projectId) {
  const explained = await request(`/v1/projects/${projectId}/ai/explain-code`, 'POST', {
    code: "SELECT note FROM briven_website_proof ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1",
    perspective: 'Explain this pgvector nearest-neighbor query briefly.',
  });
  check(explained.res.ok && typeof explained.data.explanation === 'string' && explained.data.explanation.length > 20,
    'configured AI model explains a real project vector query');
}
async function checkBranches(projectId, pool) {
  const record = await pool.query('SELECT engine_project_id AS id FROM projects WHERE id=$1', [projectId]);
  const engineProjectId = record.rows[0].id;
  const base = `/v1/control/projects/${engineProjectId}`;
  const listed = await request(base + '/branches');
  check(listed.res.ok, 'owner lists engine branches through website gateway');
  if (!listed.data.some(branch => branch.name === 'stage-acceptance')) {
    check((await request(base + '/branches', 'POST', { name: 'stage-acceptance', parent: 'main' })).res.ok,
      'owner creates database branch through website gateway');
  }
  const computes = await request(base + '/computes');
  check(computes.res.ok, 'owner lists project computes');
  if (!computes.data.some(compute => compute.branch === 'stage-acceptance')) {
    check((await request(base + '/branches/stage-acceptance/computes', 'POST')).res.ok, 'branch compute starts');
  }
  const connection = await request(base + '/branches/stage-acceptance/connection');
  check(connection.res.ok && connection.data.environment === 'customer', 'website issues a branch-scoped customer connection');
  hidden.push(connection.data.uri);
  const uri = new URL(connection.data.uri);
  check(uri.username === `briven_${engineProjectId}` && uri.searchParams.get('options') === `endpoint=ep-${engineProjectId}-stage-acceptance`,
    'branch credential preserves tenant and requested branch binding');
  uri.hostname = process.env.BRIVEN_ENGINE_PROXY_HOST ?? 'briven-customer-db';
  uri.searchParams.delete('sslmode');
  const { default: pg } = await import('/app/apps/api/node_modules/pg/lib/index.js');
  const client = new pg.Client({ connectionString: uri.toString(), ssl: {
    ca: readFileSync(process.env.BRIVEN_ENGINE_CA_FILE, 'utf8'), rejectUnauthorized: true,
  }, connectionTimeoutMillis: 10000 });
  client.on('error', () => {});
  try {
    await client.connect();
    const vectors = await client.query("SELECT count(*)::int AS count FROM briven_website_proof WHERE embedding IS NOT NULL");
    check(vectors.rows[0].count >= 2, 'database branch retains source rows and vectors');
    await client.query("INSERT INTO briven_website_proof VALUES(9,'branch-only','[0,0,1]') ON CONFLICT(id) DO NOTHING");
    const main = await request(`/v1/projects/${projectId}/studio/query`, 'POST', { sql: 'SELECT count(*)::int AS count FROM briven_website_proof WHERE id=9' });
    check(main.res.ok && main.data.rows[0].count === 0, 'branch writes remain isolated from main compute');
  } finally { await client.end(); }
}
try {
  acceptance: {
  const { getDb, getPool, closeDb: close } = await import('/app/apps/api/src/db/client.ts');
  closeDb = close;
  const { eq } = await import('/app/apps/api/node_modules/drizzle-orm/index.js');
  const { users } = await import('/app/apps/api/src/db/schema.ts');
  const before = await getPool().query('SELECT email FROM users');
  check(before.rows.every(row => row.email === owner.email), 'fresh control database has no legacy accounts');
  const { auth } = await import('/app/apps/api/src/lib/auth.ts');
  if (before.rows.length === 0) {
    const result = await auth.api.signUpEmail({ body: { ...owner, name: 'Briven owner' }, headers: new Headers({ origin: 'https://briven.tech' }) });
    check(result.user?.email === owner.email, 'fresh owner account created through platform Auth');
  }
  await getDb().update(users).set({ isAdmin: true, emailVerified: true }).where(eq(users.email, owner.email));
  const login = await request('/v1/auth/sign-in/email', 'POST', owner);
  check(login.res.ok && login.data?.user?.email === owner.email, 'owner password login through staged API');
  cookie = login.res.headers.getSetCookie().map(value => value.split(';')[0]).join('; ');
  hidden.push(cookie, login.data?.token);
  check(!!cookie && (await request('/v1/me')).res.ok, 'authenticated owner session opens dashboard API');
  check((await request('/v1/internal/projects/unknown/database-connection', 'GET', undefined, false)).res.status === 401, 'private broker rejects public callers');
  check((await request('/v1/auth-core/fdi/totp/setup', 'POST', {})).res.status === 503, 'unfinished customer Auth enrollment stays unavailable');

  const vault = await import('/app/apps/api/src/services/auth-core/db.ts');
  closeEnginePool = vault.closeEnginePool;
  const { bootstrapBrivenEngineSchema } = await import('/app/apps/api/src/services/auth-core/schema.ts');
  await bootstrapBrivenEngineSchema();
  const authRows = await vault.getEnginePool().query('SELECT count(*)::int AS count FROM be_users');
  check(authRows.rows[0].count === 0, 'fresh Auth schema initialized with no imported users');

  let listing = await request('/v1/projects');
  check(listing.res.ok, 'owner project listing available');
  const list = listing.data.projects ?? listing.data;
  let project = list.find(row => row.slug === 'serverless-acceptance');
  if (!project) {
    const created = await request('/v1/projects', 'POST', { name: 'Serverless acceptance', slug: 'serverless-acceptance' });
    check(created.res.ok, 'website creates project and ready engine compute');
    project = created.data.project ?? created.data;
  }
  check(!!project.id, 'website project ID recorded');
  const projectId = project.id;
  proofProjectId = projectId;
  if (process.env.BRIVEN_ACCEPTANCE_PHASE === 'ai') {
    await checkAi(projectId);
    console.log(JSON.stringify({ phase: 'staged-ai', result: 'pass' }));
    break acceptance;
  }
  if (process.env.BRIVEN_ACCEPTANCE_PHASE === 'branches') {
    await checkBranches(projectId, getPool());
    console.log(JSON.stringify({ phase: 'staged-branches', result: 'pass' }));
    break acceptance;
  }
  if (process.env.BRIVEN_ACCEPTANCE_PHASE === 'public-connections') {
    const record = await getPool().query('SELECT engine_project_id AS id FROM projects WHERE id=$1', [projectId]);
    const engineProjectId = record.rows[0].id;
    const shell = await request(`/v1/projects/${projectId}/db/shell-token`, 'POST');
    check(shell.res.ok, 'owner obtains customer shell connection');
    const main = await request(`/v1/control/projects/${engineProjectId}/branches/main/connection`);
    const branch = await request(`/v1/control/projects/${engineProjectId}/branches/stage-acceptance/connection`);
    check(main.res.ok && branch.res.ok, 'owner obtains main and branch customer connections');
    for (const [label, value, branchName] of [['shell', shell.data.dsn, 'main'], ['main', main.data.uri, 'main'], ['branch', branch.data.uri, 'stage-acceptance']]) {
      hidden.push(value);
      const uri = new URL(value);
      check(uri.hostname === 'briven.tech' && uri.port === '5432' && uri.username === `briven_${engineProjectId}` && uri.searchParams.get('sslmode') === 'verify-full' && uri.searchParams.get('options') === `endpoint=ep-${engineProjectId}${branchName === 'main' ? '' : '-' + branchName}`,
        `${label} customer response publishes verified public TLS and exact endpoint binding`);
    }
    check(main.res.headers.get('cache-control')?.includes('no-store') && branch.res.headers.get('cache-control')?.includes('no-store'),
      'customer gateway connection responses prohibit caching');
    console.log(JSON.stringify({ phase: 'staged-public-connections', result: 'pass' }));
    break acceptance;
  }
  const query = async sql => request(`/v1/projects/${projectId}/studio/query`, 'POST', { sql });
  let result = await query("CREATE TABLE IF NOT EXISTS public.briven_website_proof(id integer PRIMARY KEY, note text, embedding vector(3)); INSERT INTO public.briven_website_proof VALUES (1,'nearest','[1,0,0]'),(2,'farther','[0,1,0]') ON CONFLICT (id) DO NOTHING");
  check(result.res.ok, 'Studio vector table creation and writes');
  result = await query("SELECT note FROM public.briven_website_proof ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1");
  check(result.res.ok && result.data.rows[0].note === 'nearest', 'Studio pgvector similarity search');
  check(!(await query('SELECT * FROM public._briven_meta')).res.ok, 'Studio cannot read platform metadata');
  check(!(await query("INSERT INTO briven_website_proof VALUES (3,'rollback','[0,0,1]'); SELECT 1/0")).res.ok, 'failed Studio write transaction rejected');
  result = await query('SELECT count(*)::int AS count FROM briven_website_proof WHERE id=3');
  check(result.data.rows[0].count === 0, 'failed Studio transaction rolls back its write');

  if (process.env.BRIVEN_ACCEPTANCE_PHASE !== 'core') await checkAi(projectId);

  const { engineServiceLease, engineCustomerLease, engineCa } = await import('/app/apps/api/src/services/engine-database.ts');
  const db = await import('/app/apps/api/src/db/data-plane.ts');
  closeDataPlane = db.closeDataPlane;
  await db.runInProjectPlatformDatabase(projectId, tx => tx.unsafe("INSERT INTO _briven_meta VALUES ('website_proof','true') ON CONFLICT (key) DO UPDATE SET value=excluded.value"));
  check((await db.runInProjectPlatformDatabase(projectId, tx => tx.unsafe("SELECT value FROM _briven_meta WHERE key='website_proof'")))[0].value === true, 'trusted API metadata uses platform role');
  const lease = await engineServiceLease(projectId, 'runtime');
  const customer = await engineCustomerLease(projectId);
  hidden.push(lease.uri, customer.uri);
  await engineCustomerLease(projectId);
  result = await query('SELECT count(*)::int AS count FROM briven_website_proof');
  check(result.res.ok && result.data.rows[0].count >= 2, 'customer key rotation preserves API queries');

  const source = `import {query} from '@briven/cli/server'; export const vectorRows=query(async(ctx)=>{let secretBlocked=false;try{ctx.env.BRIVEN_RUNTIME_SHARED_SECRET}catch{secretBlocked=true};return {rows:await ctx.db('briven_website_proof').select().orderBy('id'),secretBlocked}});`;
  const snapshot = { version: 1, tables: { briven_schema_proof: { columns: {
    id: { sqlType: 'integer', nullable: false, primaryKey: true, unique: false },
    embedding: { sqlType: 'vector(3)', nullable: true, primaryKey: false, unique: false },
  }, indexes: [] } } };
  const deploy = await request(`/v1/projects/${projectId}/deployments`, 'POST', {
    functionNames: ['vectorRows'], bundle: { 'vectorRows.ts': source }, functionCount: 1, schemaSnapshot: snapshot,
  });
  console.log(JSON.stringify({ deploymentStatus: deploy.data?.deployment?.status ?? deploy.data?.status, deploymentError: deploy.data?.deployment?.errorMessage ?? deploy.data?.error }));
  check(deploy.res.ok && deploy.data.deployment?.status === 'succeeded', 'function and vector schema deployment succeeds');
  check((await query('ALTER TABLE briven_schema_proof ADD COLUMN IF NOT EXISTS note text')).res.ok, 'customer owns tables created by schema deployment');
  const invoked = await request(`/v1/projects/${projectId}/functions/vectorRows`, 'POST', {});
  check(invoked.res.ok && invoked.data.ok && invoked.data.value?.rows?.length >= 2, 'Deno function queries its project compute');
  check(invoked.data.value.secretBlocked === true, 'customer function cannot access host broker secret');


  const { PollManager } = await import('/app/apps/realtime/src/poll-manager.ts');
  const { SubscriptionRegistry } = await import('/app/apps/realtime/src/subscription-registry.ts');
  const registry = new SubscriptionRegistry();
  const channel = `briven_proj_${projectId.replace(/[^a-zA-Z0-9]/g, '_').toLowerCase()}_briven_website_proof`;
  registry.attach('staged-proof', channel);
  const fired = [];
  manager = new PollManager(registry, async channel => { fired.push(channel); }, 500, undefined, {
    lease: (id, refresh) => engineServiceLease(id, 'runtime', refresh), ca: engineCa,
  });
  await manager.addProject(projectId, 'briven_website_proof');
  check((await query("UPDATE briven_website_proof SET note='nearest' WHERE id=1")).res.ok, 'application write with native notification trigger');
  for (let attempt = 0; attempt < 100 && fired.length === 0; attempt++) await new Promise(resolve => setTimeout(resolve, 25));
  check(fired.includes(channel), 'verified TLS realtime client receives committed table notification');

  const issued = await request(`/v1/projects/${projectId}/api-keys`, 'POST', {
    name: 'Staged realtime acceptance', role: 'developer', expiresInDays: 1,
  });
  check(issued.res.ok && issued.data.plaintext?.startsWith('brk_'), 'owner issues project-scoped realtime key');
  proofKeyId = issued.data.key.id;
  hidden.push(issued.data.plaintext);
  const frames = [];
  socket = new WebSocket('ws://briven-serverless-realtime:3004/v1/subscribe?token=' + encodeURIComponent(issued.data.plaintext));
  socket.addEventListener('open', () => socket.send(JSON.stringify({
    type: 'subscribe', subscriptionId: 'staged-realtime', projectId, functionName: 'vectorRows', args: {},
  })));
  socket.addEventListener('message', event => {
    const frame = JSON.parse(String(event.data));
    if (frame.type !== 'hello') frames.push(frame);
  });
  const waitForFrames = async count => {
    for (let attempt = 0; attempt < 400 && frames.length < count; attempt++) await new Promise(resolve => setTimeout(resolve, 50));
  };
  await waitForFrames(1);
  console.log(JSON.stringify({ realtimeFrameType: frames[0]?.type, realtimeCode: frames[0]?.code, realtimeMessage: frames[0]?.message }));
  check(frames[0]?.type === 'data' && frames[0]?.ok && frames[0]?.value?.rows?.length >= 2,
    'deployed realtime service authenticates project key and returns function query');
  check((await query("UPDATE briven_website_proof SET note='nearest' WHERE id=1")).res.ok, 'committed write for deployed realtime subscription');
  await waitForFrames(2);
  check(frames[1]?.type === 'data' && frames[1]?.ok && frames[1]?.value?.rows?.length >= 2,
    'deployed realtime WebSocket receives database change and reruns function');
  const ready = await request('/ready');
  check(ready.res.ok && ready.data.status === 'ready', 'staged API dependency readiness');
  const web = await fetch('http://briven-serverless-website-kvm2-q76ulk-web-1:3000/dashboard', {
    headers: { cookie, host: 'briven.tech' }, redirect: 'manual', signal: AbortSignal.timeout(30_000),
  });
  check(web.status === 200, 'owner dashboard renders against fresh staged API');
  console.log(JSON.stringify({ phase: process.env.BRIVEN_ACCEPTANCE_PHASE === 'core' ? 'staged-website-core' : 'staged-website', result: 'pass', projectId, engineProjectId: lease.engineProjectId }));
  }
} catch (error) {
  let message = String(error?.message ?? error);
  for (const secret of hidden.filter(Boolean)) message = message.replaceAll(secret, '[redacted]');
  message = message.replace(/postgres(?:ql)?:\/\/[^\s"']+/g, '[redacted database URI]');
  console.log(JSON.stringify({ result: 'fail', error: message.slice(0, 1000) }));
  process.exitCode = 1;
} finally {
  socket?.close();
  if (proofKeyId) await request(`/v1/projects/${proofProjectId}/api-keys/${proofKeyId}`, 'DELETE').catch(() => {});
  await manager?.close().catch(() => undefined);
  await closeDataPlane?.().catch(() => undefined);
  await closeEnginePool?.().catch(() => undefined);
  await closeDb?.().catch(() => undefined);
}
process.exit(process.exitCode ?? 0);
