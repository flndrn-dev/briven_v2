import {readFileSync,writeFileSync} from 'node:fs';
import {createHmac,randomBytes,createHash} from 'node:crypto';
import {join} from 'node:path';
import pg from 'pg';
import {createInterface} from 'node:readline';
const commands=createInterface({input:process.stdin});
const instructions=commands[Symbol.asyncIterator]();
async function instruction(wanted){const next=await instructions.next();if(next.done||JSON.parse(next.value).instruction!==wanted)throw Error('fixed fixture instruction required');}
const root=process.env.BRIVEN_FIXTURE_ROOT;
if(process.env.GITHUB_ACTIONS!=='true'||!root?.includes('/briven-durability-'))throw Error('Owned CI fixture required');
const action=process.argv[2];
if(!['seed','exercise','prepare-interruption','interrupt','final'].includes(action))throw Error('Fixed stage required');
const config=JSON.parse(readFileSync(join(root,'client-private.json'),'utf8'));
const journal=join(root,'saved-private.json');
const state=action==='seed'?{project:null,source:null,copies:[],restores:[],computes:[]}:JSON.parse(readFileSync(journal,'utf8'));
const clients=[];
const report=check=>console.log(JSON.stringify({check,result:'pass'}));
const check=(value,message)=>{if(!value)throw Error(message);report(message);};
const save=()=>writeFileSync(journal,JSON.stringify(state),{mode:0o600});
const token=(role='owner',scope,org='org_saved_copies')=>{
 const now=Math.floor(Date.now()/1000);
 const header=Buffer.from(JSON.stringify({alg:'HS256',typ:'JWT'})).toString('base64url');
 const body=Buffer.from(JSON.stringify({iss:'briven-api',aud:'briven-control',sub:'saved_copy_fixture',org_id:org,role,iat:now,exp:now+60,...(scope?{database_removal:scope}:{})})).toString('base64url');
 return header+'.'+body+'.'+createHmac('sha256',config.identitySecret).update(header+'.'+body).digest('base64url');
};
async function api(path,method='GET',body,assertion=token()){
 const response=await fetch(config.origin+path,{method,headers:{authorization:'Bearer '+assertion,'content-type':'application/json'},...(body?{body:JSON.stringify(body)}:{}),signal:AbortSignal.timeout(150000)});
 return {status:response.status,data:await response.json().catch(()=>null)};
}
const scope=job=>({project_id:state.project.id,branch:job.name,timeline_id:job.id,action:'remove_saved_branch'});
const path=job=>`/v1/projects/${state.project.id}/branches/${job.name}/reserved-removal`;
const remove=(job,assertion=token('admin',scope(job)),body={timelineId:job.id})=>api(path(job),'DELETE',body,assertion);
async function idleRemoval(job){
 for(let n=0;n<20;n++){
  const result=await remove(job);
  if(result.data?.outcome!=='blocked'||result.data?.reason!=='active_connections')return result;
  await new Promise(resolve=>setTimeout(resolve,100));
 }
 throw Error('owned client did not finish disconnecting');
}

async function branch(prefix,parent='main'){
 const body={name:prefix+'-'+randomBytes(12).toString('hex'),parent,timelineId:randomBytes(16).toString('hex')};
 if(prefix==='sc'){
  const c=await connect('main');body.startLsn=(await c.query('SELECT pg_current_wal_flush_lsn()::text AS lsn')).rows[0].lsn;await c.end();
 }
 const response=await api(`/v1/projects/${state.project.id}/branches`,'POST',body);
 check(response.status===201&&response.data.id===body.timelineId,'normal engine branch operation preserves its exact identity');
 return {...response.data,input:body};
}
async function connect(name){
 const result=await api(`/v1/projects/${state.project.id}/branches/${name}/connection`);
 check(result.status===200&&result.data.environment==='customer','customer lease is issued through the real engine workflow');
 const uri=new URL(result.data.uri);uri.searchParams.delete('sslmode');
 check(decodeURIComponent(uri.username)==='briven_'+state.project.id,'connection has the project customer role');
 const client=new pg.Client({connectionString:uri.toString(),ssl:{ca:config.ca,rejectUnauthorized:true,servername:'briven-customer-db'},connectionTimeoutMillis:90000,query_timeout:30000});
 client.on('error',()=>{});clients.push(client);await client.connect();
 check(client.connection.stream.encrypted&&client.connection.stream.authorized,'customer transport verifies the real TLS hostname');
 return client;
}
async function hash(c){
 const rows=(await c.query('SELECT id,payload,embedding::text AS vector FROM saved_records ORDER BY id')).rows;
 const indexes=(await c.query("SELECT indexname,indexdef FROM pg_indexes WHERE tablename='saved_records' ORDER BY indexname")).rows;
 return {rows:rows.length,hash:createHash('sha256').update(JSON.stringify({rows,indexes})).digest('hex')};
}
try{
 if(action==='seed'){
  const created=await api('/v1/projects','POST',{name:'saved-copy-fixture'});
  check(created.status===201,'creates one isolated source project');state.project=created.data;save();
  const c=await connect('main');
  await c.query('CREATE TABLE saved_records(id int PRIMARY KEY,payload text NOT NULL,embedding vector(3) NOT NULL)');
  await c.query("INSERT INTO saved_records SELECT i,'original '||i,ARRAY[i::real,1,0]::vector FROM generate_series(1,42) i");
  await c.query('CREATE INDEX saved_vector_hnsw ON saved_records USING hnsw(embedding vector_cosine_ops)');
  state.saved=await hash(c);
  const copy=await branch('sc');state.copies.push(copy);save();
  await c.query("INSERT INTO saved_records VALUES(43,'later source write','[4,3,1]')");state.source=await hash(c);await c.end();
  const restored=await branch('rc',copy.name);state.restores.push(restored);save();
  const computers=await api(`/v1/projects/${state.project.id}/computes`);state.computes=computers.data;save();
 }else if(action==='exercise'){
  const copy=state.copies[0],restored=state.restores[0];
  const started=await api(`/v1/projects/${state.project.id}/branches/${restored.name}/computes`,'POST',{});
  check([200,201].includes(started.status),'starts only the named restored compute');
  const c=await connect(restored.name);
  check(JSON.stringify(await hash(c))===JSON.stringify(state.saved),'restore preserves saved rows vectors and indexes before later source writes');
  await c.query("INSERT INTO saved_records VALUES(44,'independent restored write','[4,4,1]')");
  const parent=await remove(copy);
  check(parent.status===200&&parent.data.outcome==='blocked'&&parent.data.reason==='dependent_branch','parent removal refuses a live restored dependency before accepting intent');
  check((await api(`/v1/projects/${state.project.id}/branches`,'POST',copy.input)).status===200,'preflight parent refusal leaves its identity usable');
  console.log(JSON.stringify({marker:'hide-owned-pidfiles'}));await instruction('pidfiles-hidden');
  const orphan=await remove(restored);
  console.log(JSON.stringify({marker:'restore-owned-pidfiles'}));await instruction('pidfiles-restored');
  check(orphan.status===200&&orphan.data.outcome==='blocked'&&orphan.data.reason==='identity_unavailable','missing PID files cannot disguise live bound compute processes as stopped');
  const busy=await remove(restored);
  check(busy.status===200&&busy.data.outcome==='blocked'&&busy.data.reason==='active_connections','removal protects the actual open customer connection');
  check((await c.query('SELECT count(*)::int AS n FROM saved_records')).rows[0].n===43,'refused removal preserves the active session and branch data');
  await c.query('BEGIN');await c.query("INSERT INTO saved_records VALUES(45,'prepared only','[4,5,1]')");await c.query("PREPARE TRANSACTION 'saved_copy_acceptance'");await c.end();
  const prepared=await idleRemoval(restored);
  check(prepared.status===200&&prepared.data.outcome==='blocked'&&prepared.data.reason==='prepared_transactions','removal protects real prepared work after its client disconnects');
  const rollback=await connect(restored.name);await rollback.query("ROLLBACK PREPARED 'saved_copy_acceptance'");await rollback.end();
  check((await remove(restored,token())).status===403,'ordinary owner identity cannot call private removal');
  check((await remove(restored,token('viewer',scope(restored)))).status===401,'viewer cannot acquire removal authority');
  check((await remove(restored,token('developer',scope(restored)))).status===401,'developer cannot acquire removal authority');
  check((await remove(restored,token('admin',scope(restored),'org_foreign'))).status===404,'foreign organization cannot remove the reserved branch');
  check((await remove(restored,token('admin',scope(restored)),{timelineId:'f'.repeat(32)})).status===403,'removal body cannot change the granted timeline');
  check((await api('/v1/projects','GET',undefined,token('admin',scope(restored)))).status===403,'removal grant cannot read ordinary customer routes');
  for(const target of [restored,copy]){
   const removed=await idleRemoval(target);check(removed.status===200&&removed.data.outcome==='removed','actual leaf removal confirms storage WAL and stopped compute cleanup');
   check((await remove(target)).data.outcome==='removed','duplicate removal reconciles the same tombstone');
   check((await api(`/v1/projects/${state.project.id}/branches`,'POST',target.input)).status===409,'removed timeline cannot be recreated through the API');
  }
  const main=await connect('main');check(JSON.stringify(await hash(main))===JSON.stringify(state.source),'source rows vectors and indexes are unchanged after branch removal');await main.end();
 }else if(action==='prepare-interruption'){
  state.interrupted=await branch('sc');state.copies.push(state.interrupted);save();
 }else if(action==='interrupt'){
  console.log(JSON.stringify({marker:'removal-dispatched'}));
  let interrupted=false;
  try{const result=await remove(state.interrupted);interrupted=result.status===503;}catch{interrupted=true;}
  check(interrupted,'actual engine interruption never reports completed removal');
 }else if(action==='final'){
  const target=state.interrupted;
  const result=await remove(target);check(result.status===200&&result.data.outcome==='removed','restarted engine resumes the same partial storage cleanup');
  for(const removed of [...state.copies,...state.restores]){
   check((await api(`/v1/projects/${state.project.id}/branches`,'POST',removed.input)).status===409,'durable tombstone prevents resurrection after a full engine restart');
  }
  const main=await connect('main');check(JSON.stringify(await hash(main))===JSON.stringify(state.source),'all acknowledged source data vectors and index definitions survive interrupted cleanup and restart');
  await main.query("SET enable_seqscan=off");
  const search=await main.query("EXPLAIN SELECT id FROM saved_records ORDER BY embedding <=> '[1,1,0]'::vector LIMIT 3");
  check(search.rows.some(r=>r['QUERY PLAN'].includes('saved_vector_hnsw')),'preserved vector index remains usable after restart');
  await main.end();
 }
 report('saved-copy engine phase completed');
}catch(error){console.log(JSON.stringify({check:'saved-copy engine assertion',result:'fail',phase:action,errorClass:error?.name,reason:/^[a-zA-Z0-9 ,;().-]{1,160}$/.test(error?.message||'')?error.message:'details suppressed'}));process.exitCode=1;}
finally{commands.close();await Promise.allSettled(clients.map(c=>c.end()));}
