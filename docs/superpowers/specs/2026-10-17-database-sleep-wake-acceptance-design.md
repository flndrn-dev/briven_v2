# Automatic database sleep/wake — final acceptance and controlled activation

## Objective and existing contract

Close the selected automatic database sleep/wake gap, currently estimated at 85%, against the actual deployed release. Continue the accepted lifecycle contract in `../plans/2026-10-09-graceful-compute-lifecycle.md` and the hosted-engine contract in `2026-09-28-sprint-3-serverless-cutover-design.md`. This document specifies the remaining failure checks, operating observation, and activation boundary. Execution is native, as previously requested.

This feature is explicit owner/admin opt-in for named existing endpoints. Completion means that this supported operating mode passes its acceptance gates and is activated for the reviewed endpoint. It does not mean that all customer databases have been enrolled. Existing customer databases require a separate explicit enrollment decision.

## Verified baseline — 17 October

Read-only inspection of server `briven.tech / 187.77.183.190` found the current engine healthy, `BRIVEN_COMPUTE_LIFECYCLE_ENABLED=false`, endpoint allowlist `[]`, and no rows in `briven_compute_lifecycle.endpoints`.

Running engine source: `07711c126016e0b8c7958a62e5fc95d22a844bd2`.

Manifest: `sha256:359ca9530ceb35f77ee83d7dea1b1d409ccd9647810cd97df35843ba8ab3bbdc`.

Actual image configuration: `sha256:94578897365a549c114be6578de1046807612954b913f699d70fad283a9231c6`.

The current implementation contains a 30-second scheduler, durable generation-checked records, smart shutdown, exact endpoint/project/timeline binding, bounded activity inspection, process identity checks, and serialized authenticated demand wake. Existing tests cover policy decisions and reopened stores. Historical real engine trials are useful background; they do not substitute for current-release restart acceptance.

`start-private.sh` supervises the API and proxy together. Killing the production API can terminate the engine container and affect other databases. Abrupt process and container restart trials therefore belong in disposable Linux infrastructure. No shared live API/container kill is authorized by this design.

## Approaches considered

1. **Recommended: accept the deployed immutable image in isolation, then run a named live canary.** This proves real restart behavior with no production fault injection and avoids an unnecessary application rebuild. If a product defect is found, add a regression, repair it within the existing contract, pass exact-source Linux acceptance, and deploy through the reviewed native target before repeating live acceptance.
2. Rebuild the engine before testing. This adds release work and changes the accepted artifact without evidence that application changes are needed.
3. Enable all customer endpoints immediately. This expands automatic authority before observing the final release and violates the explicit enrollment contract.

## Invariants

- Default idle window remains 300 seconds; accepted configured bounds remain 60–3600 seconds; scheduler cadence remains 30 seconds.
- Any other client backend protects the compute, including idle, idle-in-transaction, and misleading application names. Prepared transactions also protect it. Failed or missing activity inspection prohibits sleep.
- Only PostgreSQL smart shutdown is used. Existing sessions can finish; there is no fast fallback, forced client disconnect, or compute-controller termination to make a trial pass.
- Persist `stopping` before dispatch and `starting` before launching. Reconcile actual readiness and bound process identity before further action. TCP refusal alone never proves process exit.
- Authenticated concurrent demand coalesces to one startup. Do not replay client SQL. An unexpected exit or incomplete wake remains a durable review condition rather than an automatic crash restart loop.
- Retain lifecycle records and generations across restarts and rollback. Never manually rewrite live lifecycle phases, generations, failures, or project deletion flags.
- Preserve all saved copies, restored branches, current Auth fixtures, customer data, routing, certificates, admission limits, resource controls, volumes, and existing worker recovery authority/history.
- Preserve log limits of 10m × 3. No Docker socket, privileged container, host PID namespace, writable cgroup mount, deployment-host build, daemon inventory polling, or registry polling is added.

## Disposable Linux acceptance

Use a fresh owned CI Compose namespace, pinned dependencies, internal networking, no public ports, and synthetic credentials that never appear in logs. Match both the engine manifest and actual image configuration. Record harness source independently from application source. Reuse the existing durability fixture and its image, namespace, mount, network, and cleanup guards.

The required cases are:

1. Reject anonymous, viewer, foreign-organization, invalid idle-window, non-allowlisted, wrong-binding, and duplicate/stale state mutations. Assert denied requests leave lifecycle state unchanged.
2. Prove the actual scheduler preserves open transactions, idle sessions, and prepared work beyond the configured idle window. Commit normally, disconnect normally, and observe real sleep. Activity-inspection failure must keep the compute awake.
3. Open eight simultaneous hostname-verified TLS connections to the sleeping compute. Verify one actual compute startup, one increment of the durable startup counter, unchanged endpoint/timeline identity, exact committed rows/vectors, and working indexed vector search. Repeat at least three complete sleep/wake cycles.
4. Restart the actual control API while a record is sleeping. It must remain sleeping without a compute process until authenticated demand arrives. Restart while running and verify the existing compute identity is retained without duplicate launch.
5. Interrupt real transitions after durable `stopping` and `starting` records. A fixed test-only CLI barrier may hold dispatch or completion around the real bundled CLI; it must not replace lifecycle decisions or fabricate successful process outcomes. Prove unacknowledged smart dispatch reconciles safely, a live draining session is preserved, an already-ready startup reconciles without another launch, and a start with no surviving process becomes review-required.
6. Restart the entire owned engine container while sleeping; persistent lifecycle/catalog data and sleeping status must survive. Exercise unexpected compute exit separately and prove durable review refusal without an automatic restart loop. Resolve only the synthetic fixture through the existing explicit operator procedure and verified owner/admin enrollment.
7. Check capacity refusal and ambiguous/mismatched process identity without spawning a duplicate process. Preserve binding checks and removed/saved-timeline protections.
8. Exercise rollback: disable the enrolled policy, finish any graceful drain, wake normal sleeping state with the current release, and prove no subsequent automatic sleep over an idle window. Retain records; clean up all owned processes, sessions, volumes, networks, and private credential files.

Actual API-only restart trials may temporarily pause the owned CI shell supervisor so it cannot turn a child restart into an engine shutdown. Guard PID, executable, parent and namespace before every signal; restart the same actual API binary with the same configuration. This mechanism is restricted to the disposable CI namespace. Fixed deadlines and cleanup must release or remove every owned paused process even after failure.

No skipped acceptance case counts as a pass. Product behavior changes require passing regressions and a new accepted release. Resource stress/OOM acceptance remains its own production gap; these checks preserve the existing resource limits.

## Bounded live trial

The sole candidate is the previously authorized synthetic main database:

- Website project: `p_01M4V8KYS62GQ49AQQ2NQ8PAQJ`.
- Organization: `org_01M4V8KY00X11YHKYXF8YT3WNC`.
- Engine tenant: `25ab618c43b6b9fc102a4dfb111d637e`.
- Expected main endpoint: `ep-25ab618c43b6b9fc102a4dfb111d637e`; verify its actual timeline, definition and ownership before enrollment.

Inventory all active computes and sessions before any native configuration rollout. Stop if an unrelated active workload, unknown process identity, or capacity conflict would be interrupted; prepare a concrete safe maintenance decision for the owner. Do not free capacity by stopping customer databases. Do not enroll the two retained saved/restore branches.

Verify the exact registered native Dokploy target: Briven / production, engine Compose `tzky4csBhT4ZAGKdhf0R9`, server `briven.tech / 187.77.183.190`, repository `flndrn-dev/briven_v2`, and all three existing Compose overlays. Scope deployment to the engine; retain the original saved deployment command afterward. Make no API/web/supervisor deployment unless required by an accepted defect fix. Back up and restore all six existing acceptance journals if an API replacement becomes necessary.

After Linux acceptance, prepare only this endpoint's allowlist and normal owner/admin enrollment. Use a 60-second window for the bounded canary, then 300 seconds for the retained configuration. Obtain the owner's final action-time confirmation before granting the automatic sleep authority. Do not display or request credentials/codes in chat.

Observe a bounded 30-minute live window: open transaction and idle-connection protection, normal disconnection, actual graceful sleep, eight concurrent authenticated TLS cold connections, and repeated normal cycles. Include at least one complete cycle at the retained 300-second idle setting. Verify independently the actual process identities, persisted state/generation/start count, exact 212 baseline rows and vector/schema hashes, and monitoring freshness. Keep test writes out of the golden table; any additional canary table must be uniquely named, journaled, and removed after verification. A prepared-transaction failure trial is isolated CI only, so it cannot leave live prepared work behind.

Do not query SQL merely to observe sleeping state; status and process checks must not create demand. Fresh per-compute SQL metrics must not wake sleeping endpoints or falsely mark planned sleep as a crash. Preserve the already accepted host/compute monitoring and observation-worker recovery settings.

## Rollback and failure handling

For a normal rollback, disable the policy through the authenticated lifecycle route while the configured endpoint remains allowed. Let an existing graceful drain finish; use normal authenticated demand to wake a safely sleeping compute. Verify ready state and zero unfinished transitions before clearing the allowlist or disabling the scheduler through native Dokploy. Retain durable records and accepted image selectors. Never roll back over a live draining process or assume an older API understands retained failures.

An unknown, failed, or interrupted live result blocks completion. Preserve the record and evidence, disable further automatic sleep where safely possible, and follow explicit operator review. Do not clear failure history, edit generations, force stop, or increase resource/recovery budgets to obtain a green result.

## Closure evidence

The selected gap reaches 100% only after immutable-image Linux acceptance, required release/deployment checks, owner-confirmed activation, bounded live observation at the retained policy, tested disable/wake rollback, exact data/security/preservation checks, session/process cleanup, scoped commits, main synchronization, and a redacted report all pass. State the active endpoint and idle window explicitly. If only the isolated trial passes and activation is pending, report the gap as still open.

**Review status:** Owner approved native execution on 17 October. Isolated Linux acceptance exposed an unproven-controller-exit defect; the scoped repair and regression are in release acceptance. Live automatic authority remains disabled pending successful gates and the required action-time confirmation.
