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
    method, headers: { origin: 'https://briven.tech', ...(cookie && authorized ? { cookie } : {}),
      ...(body === undefined ? {} : { 'content-type': 'application/json' }) },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }), signal: AbortSignal.timeout(150_000),
  });
  const data = await res.json().catch(() => null);
  return { res, data };
}
let manager;
let closeDataPlane, closeDb, closeEnginePool;
try {
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
  const query = async sql => request(`/v1/projects/${projectId}/studio/query`, 'POST', { sql });
  let result = await query("CREATE TABLE IF NOT EXISTS public.briven_website_proof(id integer PRIMARY KEY, note text, embedding vector(3)); INSERT INTO public.briven_website_proof VALUES (1,'nearest','[1,0,0]'),(2,'farther','[0,1,0]') ON CONFLICT (id) DO NOTHING");
  check(result.res.ok, 'Studio vector table creation and writes');
  result = await query("SELECT note FROM public.briven_website_proof ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1");
  check(result.res.ok && result.data.rows[0].note === 'nearest', 'Studio pgvector similarity search');
  check(!(await query('SELECT * FROM public._briven_meta')).res.ok, 'Studio cannot read platform metadata');
  check(!(await query("INSERT INTO briven_website_proof VALUES (3,'rollback','[0,0,1]'); SELECT 1/0")).res.ok, 'failed Studio write transaction rejected');
  result = await query('SELECT count(*)::int AS count FROM briven_website_proof WHERE id=3');
  check(result.data.rows[0].count === 0, 'failed Studio transaction rolls back its write');

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
  check(deploy.res.ok && deploy.data.deployment?.status === 'succeeded', 'function and vector schema deployment succeeds');
  check((await query('ALTER TABLE briven_schema_proof ADD COLUMN IF NOT EXISTS note text')).res.ok, 'customer owns tables created by schema deployment');
  const invoked = await request(`/v1/projects/${projectId}/functions/vectorRows`, 'POST', {});
  check(invoked.res.ok && invoked.data.ok && invoked.data.value?.rows?.length >= 2, 'Deno function queries its project compute');
  check(invoked.data.value.secretBlocked === true, 'customer function cannot access host broker secret');

  const explained = await request(`/v1/projects/${projectId}/ai/explain-code`, 'POST', {
    code: "SELECT note FROM briven_website_proof ORDER BY embedding <-> '[1,0,0]'::vector LIMIT 1",
    perspective: 'Explain this pgvector nearest-neighbor query briefly.',
  });
  check(explained.res.ok && typeof explained.data.explanation === 'string' && explained.data.explanation.length > 20,
    'configured AI model explains a real project vector query');

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
  const ready = await request('/ready');
  check(ready.res.ok && ready.data.status === 'ready', 'staged API dependency readiness');
  const web = await fetch('http://briven-serverless-website-kvm2-q76ulk-web-1:3000/dashboard', {
    headers: { cookie, host: 'briven.tech' }, redirect: 'manual', signal: AbortSignal.timeout(30_000),
  });
  check(web.status === 200, 'owner dashboard renders against fresh staged API');
  console.log(JSON.stringify({ phase: 'staged-website', result: 'pass', projectId, engineProjectId: lease.engineProjectId }));
} catch (error) {
  let message = String(error?.message ?? error);
  for (const secret of hidden.filter(Boolean)) message = message.replaceAll(secret, '[redacted]');
  message = message.replace(/postgres(?:ql)?:\/\/[^\s"']+/g, '[redacted database URI]');
  console.log(JSON.stringify({ result: 'fail', error: message.slice(0, 1000) }));
  process.exitCode = 1;
} finally {
  await manager?.close().catch(() => undefined);
  await closeDataPlane?.().catch(() => undefined);
  await closeEnginePool?.().catch(() => undefined);
  await closeDb?.().catch(() => undefined);
}
process.exit(process.exitCode ?? 0);
