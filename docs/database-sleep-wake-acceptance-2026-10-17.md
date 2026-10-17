# Automatic database sleep/wake acceptance — 17 October 2026

## Status

**In progress; the selected gap is not closed.** The owner approved the written design and native execution. Production automatic sleep/wake remains disabled, with an empty endpoint allowlist. A repaired engine must pass the expanded Linux gate before deployment and the required owner-confirmed activation.

Design: `superpowers/specs/2026-10-17-database-sleep-wake-acceptance-design.md`.
Plan: `superpowers/plans/2026-10-17-database-sleep-wake-final-acceptance.md`.

## Defect found and repair

Actual isolated Linux run [39053909217](https://github.com/flndrn-dev/briven_v2/actions/runs/39053909217) reproduced an unsafe wake decision in the deployed `07711c126016e0b8c7958a62e5fc95d22a844bd2` engine. PostgreSQL had stopped, but its controller remained alive and paused with its PID file missing. An authenticated demand returned 503, yet advanced the durable startup-attempt count. Missing PID files were being treated as proof that both bound processes had exited.

The repair makes `Processes::observed` require the existing bounded `/proc` absence check before returning `Observation::Absent`. A still-live bound controller now prevents a replacement attempt and prevents a false completed-sleep observation. The same existing exact executable/endpoint/data-directory checks, 16,384-process bound, and refusal on unreadable identity remain authoritative. The application diff is limited to this guard and its comment.

No production fault injection was used to discover the defect. All faulted resources belonged to disposable CI namespaces.

## Release gate

Application source: `a6dc9da1959bf4f2a64f6ae2d90ad5800fc031de`. The exact acceptance harness revision is recorded independently by each workflow and its source proof; no accepted release receipt exists yet.

The complete release workflow first proves that the application inputs and runtime Docker instructions match that application revision. Its separate CI-only Docker target runs the actual Rust lifecycle store/activity tests against fresh private bundled PostgreSQL; the stage is excluded from the runtime image. Publication is downstream of that gate, real saved-copy removal/interrupted-cleanup acceptance and full real sleep/wake/restart acceptance on the same image. Application and harness revisions are recorded separately in OCI labels, source evidence and both suite summaries. The accepted manifest and actual image configuration must be independently verified before use.

Build run [39056383712](https://github.com/flndrn-dev/briven_v2/actions/runs/39056383712) predates the required Rust/PostgreSQL CI stage and cannot qualify for release acceptance. The complete workflow must pass on the updated harness before deployment; this earlier run is retained only as build diagnostics/cache preparation.

That build completed successfully and was canceled before publication. Later diagnostic runs retained the same application source. The actual Rust/PostgreSQL and saved-copy suites passed. Fixture-only repairs addressed process inspection during graceful exit and a missing cache import in the second build step.

Run [39066780374](https://github.com/flndrn-dev/briven_v2/actions/runs/39066780374) passed the missing-controller-PID/no-replacement regression, permission/binding checks, transaction/idle/prepared-work protection, failed-inspection recovery, and all three normal sleep/eight-connection cold-wake cycles. A later preparatory wait ended in acknowledged `stopping`; a post-timeout census found both processes absent while durable `sleeping` reconciliation remained pending. That census does not prove exit by the earlier 150-second deadline. It is not a complete acceptance pass.

The revised gate separates those observations: actual exit must be independently proved within the original wait, with acknowledged recent smart dispatch, no failure, and unchanged startup/CLI counts. Only then may one fixed 35-second window cover the 30-second scheduler's durable reconciliation. No allowance applies to running, unknown, live or unacknowledged state. The failed-inspection recovery allowance remains 240 seconds, with its timing measured after both fault resolution and idle expiry.

Passive fixture-only connection/disconnection logs and existing status polling now record classified session lifetimes, activity-clock changes and evidence coverage. Raw SQL, roles, credentials and application-name text are not included in the artifact. Unpaired events are explicitly unpaired evidence, not a claim that a backend is still alive. This evidence must attribute late activity before completion is claimed.

The first candidate build, [39054357452](https://github.com/flndrn-dev/briven_v2/actions/runs/39054357452), stopped before compilation because the pinned `tonic-reflection` crate download returned HTTP 500. No candidate image was published.

## Completed supporting checks

- Real disposable local PostgreSQL: 11 Rust lifecycle/storage/activity cases and two ephemeral-monitor cases passed, including real prepared work, concurrent generation claims, reopened state, unchanged project schema compatibility, and inspection-connection closure. The temporary cluster and processes were removed.
- Three Rust resource identity/admission tests and six host resource-protocol tests passed. These are not a new kernel resource-stress acceptance claim.
- Fourteen fixture-guard, cleanup and passive-log projection tests passed locally, including process-read races, rejection of failed probes as absence, credential-file removal after inventory failure and namespace teardown when the engine never started. Eight scenarios exercising the actual Node deadline function passed without network or credentials, including a late direct response, late physical proof, changed binding and missed reconciliation.
- Focused Linux diagnostic [39055979282](https://github.com/flndrn-dev/briven_v2/actions/runs/39055979282) passed running and sleeping API restarts, actual TLS cold wake, data/index checks and cleanup against the deployed immutable image. Its suite is deliberately labeled diagnostic; it does not satisfy the expanded release gate.
- Earlier Linux run [39053531957](https://github.com/flndrn-dev/briven_v2/actions/runs/39053531957) passed three real sleep/eight-connection TLS wake cycles but later timed out waiting for sleep after an API restart. This partial result is not counted as full acceptance. The expanded harness records lifecycle flags, bound processes and backend categories for any recurrence.
- Fresh code review identified acceptance omissions. The harness now includes independent process census and actual CLI invocation counts; duplicate/stale configuration and actual Rust catalog CAS checks; a real stored-binding fault; negative resource-service admission with an exact handled-request receipt; independent cleanup attempts; sleep-time monitoring observation; and a single live-trial deadline with bounded cleanup.
- Six live-harness fault/deadline checks pass in both Node and Bun without credentials or network. Lost enrollment responses, journal failures, session cleanup failures and finalizer failures all trigger an independent normal owner disable attempt. The deadline prevents new trial requests and reserves bounded cleanup time. Remote termination checks the exact owned acceptance program and proves no matching process remains before canceling its watchdog.
- A separate normal authenticated-demand step can restore the owned guided fixture even if the primary canary fails; it compares that fixture's original readable-data/schema baseline without changing the primary lifecycle failure evidence.

The resource-service negative fixture can only report preflight and refuse admission for its exact synthetic project. It never admits or launches a process and does not claim to reproduce physical host saturation. Existing allocator tests cover the fixed global and per-project capacity boundaries.

## Live baseline and reviewed boundary

Verified native target: Briven / production, server **briven.tech / 187.77.183.190**, engine Compose `tzky4csBhT4ZAGKdhf0R9`, repository `flndrn-dev/briven_v2`, branch `main`, with all three existing private/resource/public-PostgreSQL overlays.

Current running engine configuration: `sha256:94578897365a549c114be6578de1046807612954b913f699d70fad283a9231c6` (source `07711c1…`). API and observation-worker source remains `a69f19c6507d9bcbd036574f39e1b4806f611bb0`.

The sole automatic-enrollment candidate is:

- Project `p_01M4V8KYS62GQ49AQQ2NQ8PAQJ`.
- Endpoint `ep-25ab618c43b6b9fc102a4dfb111d637e`.
- Timeline `5e29a9e0899db6a8ea8f4dc402a052dd`.
- Retained idle policy: 300 seconds, scheduler cadence: 30 seconds.

Normal scoped login/TLS baseline checks preserved all 212 committed golden rows and vectors, row hash `bc3310b7f3de375a5a5e3f6b792fc9d1a474cf6c0680d8ce7b91fd7fbc7f296b`, and schema/index hash `13dd5b5b490d10ef946f39f04e7abe3f70bb1db55d10d177da8e8ae0c54a7b7c`.

A second active compute belongs to the retained guided-project acceptance fixture, project `p_01M538Y3YNWM5NXY57Z3M0XE4G`, endpoint `ep-907b9e7fd657de8d3522a1e9772b30c4`. Its normal customer-readable schema is empty, with hash `9acd2171bc4de88c13b3652af74a5072987404cf3d34414a9d8f02b3a3c67a10`. Both active computes had zero other clients and zero prepared transactions. Any native rollout must first receive the scoped maintenance confirmation, recheck this inventory, gracefully pause only these two owned fixtures, and restore the guided fixture through normal authenticated demand. Unrelated active workloads block that rollout.

All 35 running containers and protected configuration were baselined. Customer/Auth metadata hash: `825acd1000ccd170bdb134184a60666357c4afc346ab6dbb20b579a3a477472f`. Recovery-authority/history hash: `14653f3dbaed687d8c18cb583e1ab36f67327c9aa6be2e46e0df296c8d008bf8`. Endpoint definitions, catalog, volumes, ports, certificates, resource bounds, monitoring and retained saved/restore branches are protected. The temporary native deployment command has been narrowed to `--no-deps engine`; the original command must be restored after all final pushes and terminal deployment verification.

## Remaining closure gates

Successful expanded Linux acceptance and immutable publication verification; owner-confirmed scoped maintenance/activation; native deployment; bounded live protection, repeated sleep/wake, sleep-time monitoring and disable/wake rollback; one full cycle at the retained five-minute policy; exact data/security/configuration/session cleanup; final main synchronization and operating evidence.

Neither an isolated diagnostic pass nor a published image alone closes this gap. Other customer endpoints remain outside automatic enrollment unless explicitly opted in.
