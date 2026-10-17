// Exercise the actual release-gate function without network or credentials.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('probe.mjs',import.meta.url),'utf8');
const body=source.slice(source.indexOf('async function waitPhase('),source.indexOf('async function protectedFor('));
assert(body.startsWith('async function waitPhase('));
async function scenario(mode){
 let now=0,proofAt,confirmed=0;
 const base={endpointId:'ep-'+'a'.repeat(32),projectId:'a'.repeat(32),timelineId:'b'.repeat(32),enabled:true,idleSeconds:60,lastDemand:85,starts:0,stopRequested:true,failure:null};
 const record=async()=>{
  if(mode==='delayed-status'){now=151000;return {...base,phase:'sleeping',phaseSince:151};}
  let phase=mode==='running'?'running':now>=145000?'stopping':'running';
  if(proofAt&&mode==='restarted'&&now>=proofAt+1000)phase='running';
  if(proofAt&&now>=proofAt+(mode==='late-commit'?36000:30000))phase='sleeping';
  return {...base,phase,phaseSince:phase==='stopping'?145:0,...(phase==='sleeping'&&mode==='binding-changed'?{timelineId:'c'.repeat(32)}:{})};
 };
 const marker=async(name,extra)=>{
  if(name==='inspect-stopping-exit'){
   now+=1000;
   if(mode==='no-exit')return {exited:false};
   proofAt=now;
   return {exited:true,observedAt:mode==='late-proof'?extra.deadline+1:now,record:await record()};
  }
  assert.equal(name,'confirm-sleep-reconciled');
  assert(extra.observedAt<=proofAt+35000);confirmed++;
  return {};
 };
 const wait=Function('record','marker','check','delay','Date','return ('+body+');')(
  record,marker,(value,message)=>assert(value,message),async ms=>{now+=ms},{now:()=>now});
 if(mode==='success'){
  const result=await wait('sleeping',150);
  assert.equal(result.phase,'sleeping');assert(now>150000&&now<=proofAt+35000);assert.equal(confirmed,1);
 }else{
  await assert.rejects(wait('sleeping',150));assert.equal(confirmed,0);
 }
}
for(const mode of ['success','running','no-exit','late-proof','late-commit','binding-changed','restarted','delayed-status'])await scenario(mode);
console.log(JSON.stringify({phase:'sleep-wake-deadline-guards',passed:true,cases:8,network:false,credentials:false}));
