import {readFileSync,writeFileSync} from 'node:fs';
import {createHmac,createHash} from 'node:crypto';
import {join} from 'node:path';
import {createInterface} from 'node:readline';
import pg from '../postgres-durability/node_modules/pg/lib/index.js';

const root=process.env.BRIVEN_FIXTURE_ROOT;
if(process.env.GITHUB_ACTIONS!=='true'||!root?.includes('/briven-durability-'))throw Error('Disposable CI fixture required');
const config=JSON.parse(readFileSync(join(root,'client-private.json'),'utf8'));
const path=join(root,'lifecycle-private.json');
const state={project:null,uri:null,golden:null};
const clients=new Set();
let phase='seed';
const input=createInterface({input:process.stdin});
const commands=input[Symbol.asyncIterator]();
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const report=(check,extra={})=>console.log(JSON.stringify({check,result:'pass',...extra}));
const check=(value,message)=>{if(!value)throw Error(message);report(message);};
const save=()=>writeFileSync(path,JSON.stringify(state),{mode:0o600});
async function marker(marker){
 console.log(JSON.stringify({marker}));
 let timer;
 const next=await Promise.race([commands.next(),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Owned fault instruction deadline exceeded')),160000);})]).finally(()=>clearTimeout(timer));
 if(next.done||JSON.parse(next.value).instruction!==marker)throw Error('Wrong owned fault instruction');
}
function token(role='owner',org='org_lifecycle_acceptance'){
 const now=Math.floor(Date.now()/1000),header=Buffer.from(JSON.stringify({alg:'HS256',typ:'JWT'})).toString('base64url');
 const body=Buffer.from(JSON.stringify({iss:'briven-api',aud:'briven-control',sub:'lifecycle_acceptance',org_id:org,role,iat:now,exp:now+60})).toString('base64url');
 return header+'.'+body+'.'+createHmac('sha256',config.identitySecret).update(header+'.'+body).digest('base64url');
}
async function api(path,method='GET',body,role='owner',org='org_lifecycle_acceptance'){
 const response=await fetch(config.origin+path,{method,signal:AbortSignal.timeout(150000),headers:{...(role===null?{}:{authorization:'Bearer '+token(role,org)}),'content-type':'application/json'},...(body?{body:JSON.stringify(body)}:{})});
 return {status:response.status,data:await response.json().catch(()=>null)};
}
const lifecycle=()=>`/v1/projects/${state.project.id}/branches/main/lifecycle`;
async function record(){const r=await api(lifecycle());if(r.status!==200||!r.data)throw Error('Scoped lifecycle status unavailable');return r.data;}
async function configure(enabled=true,idleSeconds=60){const r=await api(lifecycle(),'POST',{enabled,idleSeconds});check(r.status===200&&r.data.enabled===enabled&&r.data.idleSeconds===idleSeconds,'owner configures exact allowed lifecycle policy');return r.data;}
async function lease(){
 const r=await api(`/v1/projects/${state.project.id}/branches/main/connection`);
 check(r.status===200&&r.data.environment==='customer','normal project customer lease issued');
 const uri=new URL(r.data.uri);uri.searchParams.delete('sslmode');
 check(uri.hostname==='briven-customer-db'&&decodeURIComponent(uri.username)==='briven_'+state.project.id,'lease has exact project role and fixture hostname');
 state.uri=uri.toString();save();
}
async function connect(){
 const c=new pg.Client({connectionString:state.uri,ssl:{ca:config.ca,rejectUnauthorized:true,servername:'briven-customer-db'},connectionTimeoutMillis:90000,query_timeout:30000});
 c.on('error',()=>{});clients.add(c);await c.connect();
 check(c.connection.stream.encrypted&&c.connection.stream.authorized,'actual customer connection verifies TLS hostname');return c;
}
async function close(c){await c.end();clients.delete(c);}
async function checksum(c){
 const rows=(await c.query('SELECT id,payload,embedding::text AS vector FROM lifecycle_records ORDER BY id')).rows;
 const indexes=(await c.query("SELECT indexname,indexdef FROM pg_indexes WHERE tablename='lifecycle_records' ORDER BY indexname")).rows;
 return createHash('sha256').update(JSON.stringify({rows,indexes})).digest('hex');
}
async function verify(c){
 check(await checksum(c)===state.golden,'exact committed rows vectors and index definitions preserved');
 await c.query('SET enable_seqscan=off');
 const plan=await c.query("EXPLAIN SELECT id FROM lifecycle_records ORDER BY embedding <=> '[1,1,0]'::vector LIMIT 1");
 check(plan.rows.some(r=>r['QUERY PLAN'].includes('lifecycle_vector_hnsw')),'retained HNSW index remains usable');
 const nearest=(await c.query("SELECT id FROM lifecycle_records ORDER BY embedding <=> '[1,1,0]'::vector LIMIT 1")).rows[0];
 check(nearest.id===1,'indexed vector query returns expected nearest neighbor');
}
async function waitPhase(wanted,seconds=150){
 const until=Date.now()+seconds*1000;
 while(Date.now()<until){
  const r=await record();
  if(r.phase===wanted)return r;
  if(r.phase==='failed'&&wanted!=='failed')throw Error('Unexpected durable lifecycle failure');
  await delay(1500);
 }
 throw Error('Expected durable phase deadline exceeded');
}
async function protectedFor(seconds,label,allowFailure=false){
 const before=await record(),until=Date.now()+seconds*1000;
 while(Date.now()<until){const r=await record();if(r.phase!=='running'||r.starts!==before.starts||(!allowFailure&&r.failure))throw Error(label);await delay(5000);}
 report(label);
}
async function coldWake(before,number=8){
 await marker('inspect-sleep');
 const connected=await Promise.all(Array.from({length:number},()=>connect()));
 await Promise.all(connected.map(verify));
 const r=await record();
 check(r.phase==='running'&&!r.failure&&r.starts===before.starts+1&&r.timelineId===before.timelineId,'concurrent cold TLS demand produces exactly one durable startup');
 await marker('inspect-running');
 await Promise.all(connected.map(close));return r;
}
async function triggerWake(){
 const result=api(`/v1/projects/${state.project.id}/branches/main/connection`).catch(()=>({status:0}));
 return {result};
}
try{
 const made=await api('/v1/projects','POST',{name:'lifecycle-acceptance'});
 check(made.status===201&&/^[0-9a-f]{32}$/.test(made.data?.id),'normal engine API creates one disposable lifecycle project');state.project=made.data;save();
 check((await api(lifecycle(),'POST',{enabled:true,idleSeconds:60})).status===404,'empty allowlist refuses enrollment before configuration');
 await lease();let c=await connect();
 await c.query('CREATE TABLE lifecycle_records(id int PRIMARY KEY,payload text NOT NULL,embedding vector(3) NOT NULL)');
 await c.query("INSERT INTO lifecycle_records SELECT i,'acknowledged '||i,ARRAY[i::real,1,0]::vector FROM generate_series(1,42) i");
 await c.query('CREATE INDEX lifecycle_vector_hnsw ON lifecycle_records USING hnsw(embedding vector_cosine_ops)');
 state.golden=await checksum(c);save();await close(c);
 await marker('enable-config');

 phase='permissions';
 const before=(await api(lifecycle())).data;
 check(before===null,'API configuration alone creates no implicit lifecycle enrollment');
 for(const [role,org,status]of [[null,'org_lifecycle_acceptance',401],['viewer','org_lifecycle_acceptance',403],['owner','org_foreign',404]]){
  check((await api(lifecycle(),'POST',{enabled:true,idleSeconds:60},role,org)).status===status,'unauthorized lifecycle enrollment rejected');
 }
 for(const idleSeconds of [0,59,3601])check((await api(lifecycle(),'POST',{enabled:true,idleSeconds})).status===400,'invalid idle window rejected');
 check((await api(lifecycle(),'POST',{enabled:true,idleSeconds:60,endpointId:'ep-other'})).status===422,'caller cannot supply a different endpoint binding');
 check((await api(lifecycle())).data===null,'denied requests leave lifecycle enrollment unchanged');
 await lease();c=await connect();
 check((await c.query("SHOW max_prepared_transactions")).rows[0].max_prepared_transactions==='4','disposable compute supports actual prepared work');
 await configure();
 phase='missing-controller-pid';await close(c);await marker('arm-orphan-controller');await waitPhase('stopping');await delay(2000);await marker('hide-orphan-controller');
 try{
  const protectedDemand=await api(`/v1/projects/${state.project.id}/branches/main/connection`);
  check(protectedDemand.status===503,'live orphan controller with missing PID file blocks destructive wake');
  check((await record()).starts===0,'unproven controller exit allocates no replacement process');
 }finally{await marker('restore-orphan-controller');}
 const orphanSleep=await waitPhase('sleeping');await coldWake(orphanSleep);await lease();c=await connect();await configure();
 phase='transaction-protection';await c.query('BEGIN');await c.query('SELECT 1');await protectedFor(75,'open transaction survives beyond idle window');await c.query('COMMIT');
 phase='idle-session-protection';await c.query("SET application_name='briven_lifecycle_inspection'");await protectedFor(75,'idle client with inspection-like name remains protected');await verify(c);
 phase='prepared-work-protection';await c.query('BEGIN');await c.query("INSERT INTO lifecycle_records VALUES(99,'prepared only','[9,9,1]')");await c.query("PREPARE TRANSACTION 'briven_lifecycle_acceptance'");await close(c);
 await protectedFor(75,'prepared transaction prevents sleep after client disconnect');
 c=await connect();await c.query("ROLLBACK PREPARED 'briven_lifecycle_acceptance'");await verify(c);await close(c);
 phase='activity-unavailable';await marker('pause-postmaster');
 try{await protectedFor(70,'failed activity sampling keeps the real compute awake',true);check((await record()).failure==='activity_unavailable','failed real inspection is recorded as unavailable');}
 finally{await marker('resume-postmaster');}
 await lease();c=await connect();await configure();await verify(c);await close(c);

 phase='repeated-normal-cycles';
 for(let cycle=0;cycle<3;cycle++){
  await lease();const initial=await record();const asleep=await waitPhase('sleeping');
  check(asleep.starts===initial.starts&&asleep.stopRequested,'actual idle scheduler reaches sleeping without a startup');
  await coldWake(asleep);
 }
 report('three actual idle sleep and eight-connection TLS wake cycles passed',{cycles:3});

 phase='running-api-restart';await lease();c=await connect();const running=await record();await marker('restart-api');
 check((await record()).starts===running.starts,'running API restart does not duplicate compute startup');await verify(c);await close(c);
 phase='sleeping-api-restart';const asleep=await waitPhase('sleeping');await marker('restart-api');
 check((await record()).phase==='sleeping'&&(await record()).starts===asleep.starts,'sleeping API restart retains durable sleep without automatic wake');await coldWake(asleep);

 phase='interrupted-stop-dispatch';await lease();await marker('arm-before-stop');
 await marker('interrupt-before-stop');
 const stopped=await waitPhase('sleeping');await coldWake(stopped);

 phase='interrupted-smart-drain';await lease();await marker('arm-after-stop');await marker('interrupt-after-stop');
 const drained=await waitPhase('sleeping');await marker('inspect-sleep');
 c=await connect();
 check((await c.query("SELECT payload,embedding::text AS embedding FROM lifecycle_records WHERE id=1000")).rows[0]?.payload==='draining commit','transaction acknowledged during real smart drain survives wake');
 await c.query('DELETE FROM lifecycle_records WHERE id=1000');await verify(c);await close(c);
 check((await record()).starts===drained.starts+1,'draining transition restarts only on later normal demand');

 phase='interrupted-start-ready';await lease();const sleepingReady=await waitPhase('sleeping');await marker('arm-after-start');
 let pending=await triggerWake();await marker('interrupt-after-start');await pending.result;
 const reconciled=await waitPhase('running');
 check(reconciled.starts===sleepingReady.starts+1&&!reconciled.failure,'interrupted ready start reconciles without duplicate launch');
 c=await connect();await verify(c);await close(c);

 phase='interrupted-start-absent';await lease();const sleepingAbsent=await waitPhase('sleeping');await marker('arm-before-start');
 pending=await triggerWake();await marker('interrupt-before-start');await pending.result;
 const incomplete=await waitPhase('failed');
 check(incomplete.failure==='wake_incomplete'&&incomplete.starts===sleepingAbsent.starts+1,'interrupted unlaunched start becomes durable review-required failure');
 const retry=await api(`/v1/projects/${state.project.id}/branches/main/connection`);
 check(retry.status===503&&(await record()).starts===incomplete.starts,'failed wake refuses another automatic launch');
 await marker('inspect-sleep');await marker('reviewed-compute-start');await configure();await lease();c=await connect();await verify(c);await close(c);

 phase='whole-engine-sleeping-restart';const sleepingEngine=await waitPhase('sleeping');await marker('crash-sleeping-engine');
 check((await record()).phase==='sleeping'&&(await record()).starts===sleepingEngine.starts,'actual full engine restart preserves durable sleeping state');await coldWake(sleepingEngine);

 phase='ambiguous-process-identity';await lease();c=await connect();await configure();const identityBefore=await record();await marker('hide-controller-identity');
 try{check((await api(`/v1/projects/${state.project.id}/branches/main/connection`)).status===503,'ambiguous controller identity refuses demand without launching');}
 finally{await marker('restore-controller-identity');}
 check((await record()).starts===identityBefore.starts,'ambiguous identity does not increment startup count');await verify(c);await configure();await close(c);

 phase='unexpected-compute-exit';await lease();await marker('crash-owned-compute');const failed=await waitPhase('failed');
 check(failed.failure==='unexpected_exit','unexpected compute exit retains review-required failure');
 for(let n=0;n<3;n++)check((await api(`/v1/projects/${state.project.id}/branches/main/connection`)).status===503,'unexpected exit refuses repeated automatic restart');
 check((await record()).starts===failed.starts,'unexpected exit consumes no automatic wake');
 await marker('reviewed-compute-start');await configure();await lease();c=await connect();await verify(c);await close(c);

 phase='rollback-disable-and-wake';const rollbackSleep=await waitPhase('sleeping');const disabled=await configure(false);
 check(disabled.phase==='sleeping'&&!disabled.enabled,'normal owner disables policy while sleeping');await coldWake(rollbackSleep);
 await protectedFor(75,'disabled policy remains awake beyond idle window');
 await lease();c=await connect();await verify(c);await close(c);
 report('all required immutable engine lifecycle phases passed',{checksum:state.golden,starts:(await record()).starts});
}catch(error){
 console.log(JSON.stringify({check:'actual lifecycle assertion',result:'fail',phase,errorClass:error?.name,sqlstate:/^[A-Z0-9]{5}$/.test(error?.code||'')?error.code:undefined,reason:/^[a-zA-Z0-9 ,;().-]{1,180}$/.test(error?.message||'')?error.message:'details suppressed'}));process.exitCode=1;
}finally{
 input.close();
 await Promise.allSettled([...clients].map(async c=>{try{await Promise.race([c.end(),delay(3000)])}finally{c.connection.stream?.destroy()}}));
 report('all owned customer TLS connections closed');
}
