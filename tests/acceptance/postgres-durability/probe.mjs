import {readFileSync, writeFileSync} from 'node:fs';
import {createHmac, randomBytes, createHash} from 'node:crypto';
import {join} from 'node:path';
import {createInterface} from 'node:readline';
import pg from 'pg';

const root=process.env.BRIVEN_FIXTURE_ROOT;
if(process.env.GITHUB_ACTIONS!=='true'||!root?.includes('/briven-durability-'))throw Error('CI fixture only');
const action=process.argv[2];
if(!['seed','verify','crash-hold','one-safekeeper','quorum','object-outage','final'].includes(action))throw Error('Fixed action required');
const config=JSON.parse(readFileSync(join(root,'client-private.json'),'utf8'));
const journal=join(root,'probe-private.json');
let state=action==='seed'?{run:config.project,project:null,branches:[],computes:[],golden:{}}:JSON.parse(readFileSync(journal,'utf8'));
const clients=[];
const report=(check,extra={})=>console.log(JSON.stringify({check,result:'pass',...extra}));
const check=(ok,message)=>{if(!ok)throw Error(message);report(message)};
const save=()=>writeFileSync(journal,JSON.stringify(state),{mode:0o600});
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
function token(role='owner',org='org_durability'){
 const now=Math.floor(Date.now()/1000),header=Buffer.from(JSON.stringify({alg:'HS256',typ:'JWT'})).toString('base64url');
 const body=Buffer.from(JSON.stringify({iss:'briven-api',aud:'briven-control',sub:'durability_fixture',org_id:org,role,iat:now,exp:now+60})).toString('base64url');
 return header+'.'+body+'.'+createHmac('sha256',config.identitySecret).update(header+'.'+body).digest('base64url');
}
async function api(path,method='GET',body,role='owner',org='org_durability'){
 const response=await fetch(config.origin+path,{method,signal:AbortSignal.timeout(180000),headers:{authorization:'Bearer '+token(role,org),...(body?{'content-type':'application/json'}:{})},...(body?{body:JSON.stringify(body)}:{})});
 return {status:response.status,data:await response.json().catch(()=>null)};
}
async function lease(branch){
 const response=await api(`/v1/projects/${state.project.id}/branches/${branch}/connection`);
 check(response.status===200&&response.data?.environment==='customer','normal scoped customer lease issued');
 const uri=new URL(response.data.uri);
 check(uri.hostname==='briven-customer-db'&&uri.port==='5432'&&decodeURIComponent(uri.username)==='briven_'+state.project.id,'lease binds exact fixture host and customer role');
 check(Date.parse(response.data.expiresAt)>Date.now(),'customer lease has a live finite expiry');
 uri.searchParams.delete('sslmode');
 return uri.toString();
}
async function client(uri){
 const value=new pg.Client({connectionString:uri,ssl:{ca:config.ca,rejectUnauthorized:true,servername:'briven-customer-db'},connectionTimeoutMillis:90000,query_timeout:30000});
 value.on('error',()=>{});clients.push(value);await value.connect();
 check(value.connection.stream.encrypted===true&&value.connection.stream.authorized===true,'actual hostname-verified TLS transport');
 return value;
}
async function connect(branch){return client(await lease(branch))}
async function hash(value,excluded=null){
 const rows=(await value.query('SELECT id,md5(payload) AS payload_hash,embedding::text AS embedding FROM durability_records WHERE ($1::text IS NULL OR id<>$1) ORDER BY id',[excluded])).rows;
 return {hash:createHash('sha256').update(JSON.stringify(rows)).digest('hex'),rows:rows.length};
}
async function snapshot(branch,value){state.golden[branch]=await hash(value);save()}
async function verifyData(branch,value){
 const data=await hash(value);
 check(data.hash===state.golden[branch].hash&&data.rows===state.golden[branch].rows,'exact acknowledged row and vector checksum survives recovery');
 check((await value.query("SELECT count(*)::int AS count FROM durability_records WHERE id='uncommitted-crash'")).rows[0].count===0,'unfinished transaction is absent after crash');
 const settings=(await value.query("SELECT current_setting('synchronous_commit') AS commit,current_setting('synchronous_standby_names') AS standby,current_setting('fsync') AS compute_fsync")).rows[0];
 check(settings.commit==='on'&&settings.standby==='walproposer','commits wait for the durable WAL quorum');
 report('compute disk is disposable; synchronous WAL quorum is the durable boundary',{computeFsync:settings.compute_fsync});
 const index=(await value.query("SELECT i.indisvalid FROM pg_index i JOIN pg_class c ON c.oid=i.indexrelid WHERE c.relname='durability_embedding_hnsw'")).rows;
 check(index.length===1&&index[0].indisvalid,'restored branch has a valid HNSW index');
 const vector=(await value.query("SELECT embedding::text AS embedding FROM durability_records WHERE id='anchor'")).rows[0].embedding;
 await value.query('SET enable_seqscan=off');
 const plan=(await value.query('EXPLAIN (FORMAT JSON) SELECT id FROM durability_records ORDER BY embedding <=> $1::vector LIMIT 1',[vector])).rows[0]['QUERY PLAN'];
 check(JSON.stringify(plan).includes('durability_embedding_hnsw'),'nearest-neighbor query uses the retained HNSW index');
 const nearest=(await value.query('SELECT id,embedding <=> $1::vector AS distance FROM durability_records ORDER BY embedding <=> $1::vector LIMIT 1',[vector])).rows[0];
 check(nearest.id==='anchor'&&Math.abs(nearest.distance)<0.000001,'indexed vector search returns the exact expected nearest neighbor');
}
async function identities(){
 const projects=await api('/v1/projects'),branches=await api(`/v1/projects/${state.project.id}/branches`),computes=await api(`/v1/projects/${state.project.id}/computes`);
 check(projects.status===200&&projects.data.length===1&&projects.data[0].id===state.project.id,'restart preserves exactly one owned project');
 const currentBranches=branches.data.map(({name,id})=>({name,id}));
 check(branches.status===200&&JSON.stringify(currentBranches)===JSON.stringify(state.branches),'restart preserves exact branch timeline identities with no duplicates');
 const currentComputes=computes.data.map(({id,branch})=>({id,branch}));
 check(computes.status===200&&JSON.stringify(currentComputes)===JSON.stringify(state.computes),'restart preserves exact compute definitions with no duplicates');
}
async function permissions(){
 const path=`/v1/projects/${state.project.id}/branches`;
 check((await api(path,'POST',{name:'viewer-denied'},'viewer')).status===403,'viewer cannot create a writable branch');
 check((await api(path,'GET',undefined,'owner','org_foreign')).status===404,'foreign organization cannot inspect the fixture branches');
 check((await api(`/v1/projects/${state.project.id}/branches/main/connection`,'GET',undefined,'viewer')).status===403,'viewer cannot obtain a write credential');
}
async function markWal(branch,value){
 const row=(await value.query('SELECT pg_current_wal_flush_lsn()::text AS lsn')).rows[0];
 console.log(JSON.stringify({marker:'verify-quorum-flush',tenant:state.project.id,timeline:state.branches.find(v=>v.name===branch).id,lsn:row.lsn}));
 await instruction('quorum-flush-verified');
}
const input=createInterface({input:process.stdin});let instructions=[];let waiting;
input.on('line',line=>{try{const record=JSON.parse(line);if(waiting){const done=waiting;waiting=null;done(record)}else instructions.push(record)}catch{process.exitCode=1}});
async function instruction(expected){
 let timer;
 const value=await Promise.race([instructions.length?Promise.resolve(instructions.shift()):new Promise(resolve=>{waiting=resolve}),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Orchestrator instruction timed out')),90000)})]).finally(()=>clearTimeout(timer));
 check(value?.instruction===expected,'matching bounded fault-control instruction');
}
async function write(branch,value,id){
 const old=state.golden[branch],before=await hash(value);
 check(before.hash===old.hash&&before.rows===old.rows,'previous acknowledged data is intact before the new write');
 const inserted=(await value.query('INSERT INTO durability_records(id,payload,embedding) VALUES($1,$2,$3::vector) RETURNING id,payload,embedding::text',[id,'acknowledged '+id,'[4,5,6]'])).rows[0];
 check(inserted.id===id&&inserted.payload==='acknowledged '+id&&inserted.embedding==='[4,5,6]','new acknowledged record matches its exact intended payload and vector');
 const unchanged=await hash(value,id);
 check(unchanged.hash===old.hash&&unchanged.rows===old.rows,'new write preserves every previously acknowledged record');
 await snapshot(branch,value);await markWal(branch,value);
}
try{
 if(action==='seed'){
  const project=await api('/v1/projects','POST',{name:'durability-fixture'});
  check(project.status===201&&/^[0-9a-f]{32}$/.test(project.data?.id),'normal engine API creates one synthetic project');state.project=project.data;save();
  const main=await connect('main');
  check((await main.query('SELECT current_user AS role')).rows[0].role==='briven_'+state.project.id,'SQL uses exact non-administrator customer role');
  await main.query('CREATE TABLE durability_records(id text PRIMARY KEY,payload text NOT NULL,embedding vector(3) NOT NULL)');
  await main.query("INSERT INTO durability_records SELECT 'seed-'||i,repeat(md5(i::text),128),ARRAY[i::real,1,0]::vector FROM generate_series(1,1024) AS i");
  await main.query("INSERT INTO durability_records VALUES('anchor','branch checkpoint','[1,2,3]'::vector)");
  await main.query('CREATE INDEX durability_embedding_hnsw ON durability_records USING hnsw(embedding vector_cosine_ops)');
  const startLsn=(await main.query('SELECT pg_current_wal_flush_lsn()::text AS lsn')).rows[0].lsn;
  const branchInput={name:'durability-branch',parent:'main',startLsn,timelineId:randomBytes(16).toString('hex')};
  const branch=await api(`/v1/projects/${state.project.id}/branches`,'POST',branchInput);
  check(branch.status===201&&branch.data.id===branchInput.timelineId,'child branch is created at its persisted timeline identity');
  check((await api(`/v1/projects/${state.project.id}/branches`,'POST',branchInput)).status===200,'same branch operation reuses its exact existing identity');
  check((await api(`/v1/projects/${state.project.id}/branches`,'POST',{...branchInput,timelineId:randomBytes(16).toString('hex')})).status===409,'changed branch retry is refused without duplication');
  check((await api(`/v1/projects/${state.project.id}/branches/durability-branch/computes`,'POST',{})).status===201,'normal child compute is created');
  state.branches=(await api(`/v1/projects/${state.project.id}/branches`)).data.map(({name,id})=>({name,id}));
  state.computes=(await api(`/v1/projects/${state.project.id}/computes`)).data.map(({id,branch})=>({id,branch}));save();
  const child=await connect('durability-branch');
  await main.query("UPDATE durability_records SET payload='original changed',embedding='[7,8,9]' WHERE id='anchor'");
  await child.query("INSERT INTO durability_records VALUES('branch-only','child changed','[10,11,12]')");
  check((await child.query("SELECT embedding::text AS value FROM durability_records WHERE id='anchor'")).rows[0].value==='[1,2,3]','later original writes do not change the child checkpoint');
  check((await main.query("SELECT count(*)::int AS count FROM durability_records WHERE id='branch-only'")).rows[0].count===0,'child writes do not change the original');
  await snapshot('main',main);await snapshot('durability-branch',child);
  await markWal('main',main);await markWal('durability-branch',child);
  await verifyData('main',main);await verifyData('durability-branch',child);await permissions();await identities();
 }else if(action==='verify'||action==='final'){
  for(const branch of ['main','durability-branch']){
   const uri=await lease(branch),connections=await Promise.all(Array.from({length:8},()=>client(uri)));
   const outcomes=await Promise.all(connections.map(async value=>{await verifyData(branch,value);return(await hash(value)).hash}));
   check(new Set(outcomes).size===1,'eight concurrent TLS connections see identical recovered contents');
   if(action==='final')await write(branch,connections[0],branch+'-after-final-recovery');
  }
  await permissions();await identities();
 }else if(action==='crash-hold'){
  const main=await connect('main');await write('main',main,'acknowledged-immediately-before-crash');
  await main.query('BEGIN');await main.query("INSERT INTO durability_records VALUES('uncommitted-crash','must roll back','[9,9,9]')");
  console.log(JSON.stringify({marker:'crash-with-uncommitted-transaction'}));
  await instruction('engine-crashed');
  for(let i=0;i<50&&!main.connection.stream.destroyed;i++)await delay(100);
  check(main.connection.stream.destroyed,'actual crash disconnected the held transaction');
 }else if(action==='one-safekeeper'){
  const main=await connect('main'),child=await connect('durability-branch');
  console.log(JSON.stringify({marker:'ready-to-pause-one-safekeeper'}));await instruction('one-safekeeper-paused');
  for(const [branch,value]of [['main',main],['durability-branch',child]]){await write(branch,value,branch+'-one-safekeeper-unavailable');await verifyData(branch,value)}
 }else if(action==='quorum'){
  const main=await connect('main');await verifyData('main',main);await main.query('BEGIN');await main.query("INSERT INTO durability_records VALUES('quorum-pending','commit once after quorum returns','[4,5,6]')");
  console.log(JSON.stringify({marker:'ready-to-pause-majority'}));await instruction('majority-paused');
  let acknowledged=false;const commit=main.query('COMMIT').then(()=>{acknowledged=true});commit.catch(()=>{});
  await delay(5000);check(!acknowledged,'new COMMIT is not acknowledged while a safekeeper majority is unavailable');
  console.log(JSON.stringify({marker:'restore-majority'}));await instruction('majority-restored');
  await commit;check(acknowledged,'the same pending COMMIT succeeds after quorum is restored');
  check((await main.query("SELECT count(*)::int AS count FROM durability_records WHERE id='quorum-pending'")).rows[0].count===1,'restored pending commit exists exactly once without retry');
  const prior=await hash(main,'quorum-pending');
  check(prior.hash===state.golden.main.hash&&prior.rows===state.golden.main.rows,'restoring quorum preserves all previously acknowledged records');
  const pending=(await main.query("SELECT payload,embedding::text AS embedding FROM durability_records WHERE id='quorum-pending'")).rows[0];
  check(pending.payload==='commit once after quorum returns'&&pending.embedding==='[4,5,6]','pending commit preserves its exact intended payload and vector');
  await snapshot('main',main);await markWal('main',main);
 }else if(action==='object-outage'){
  const main=await connect('main'),child=await connect('durability-branch');
  console.log(JSON.stringify({marker:'ready-to-pause-object-store'}));await instruction('object-store-paused');
  for(const [branch,value]of [['main',main],['durability-branch',child]])await write(branch,value,branch+'-object-store-unavailable');
 }
 report('fixed SQL acceptance phase completed',{phase:action});
}catch(error){console.log(JSON.stringify({check:'SQL acceptance phase',result:'fail',phase:action,errorClass:error?.name,sqlstate:/^[A-Z0-9]{5}$/.test(error?.code||'')?error.code:undefined,reason:/^[a-zA-Z0-9 ,;()/.-]{1,160}$/.test(error?.message||'')?error.message:'details suppressed'}));process.exitCode=1}
finally{
 input.close();
 await Promise.all(clients.map(async value=>{try{await Promise.race([value.end(),delay(3000)])}finally{value.connection.stream?.destroy()}}));
 report('all owned SQL connections closed',{phase:action});
}
