# Worker recovery and saved copies — final acceptance

Date: 17 October 2026. Owner-approved designs, native execution and final activation.

| Selected gap | Before | Accepted | Live state |
|---|---:|---:|---|
| Automatic worker recovery | 85% | **100%** | Automatic for the tested `briven-operations-observer` only |
| Saved copies and restore | 85% | **100%** | Public manual copies and restore to a new branch enabled |

These percentages describe the two approved contracts. They do not certify the entire platform, scheduled backups, point-in-time recovery, other worker/database/host recovery, physical-device Auth acceptance or the separate credential-rotation/handover gates.

## Release identity and automated gates

- API, web and supervisor application source: `a69f19c6507d9bcbd036574f39e1b4806f611bb0`.
- Engine binary source: `07711c126016e0b8c7958a62e5fc95d22a844bd2`.
- [Website Linux acceptance](https://github.com/flndrn-dev/briven-website/actions/runs/39032554153): 98 tests, 484 assertions, zero failures/skips; real PostgreSQL 17.11, HTTP, actual React controls, API/web types/builds and focused lint passed. Local acceptance also passed.
- [Real executor interruption](https://github.com/flndrn-dev/briven-website/actions/runs/39042854199): harness `db06cf7d47de3d87512acc4cb99e6fd777daa9d9` runs the accepted API manifest without application changes. Actual executor SIGKILL/reaping, child IPC-disconnect exit, durable claim on a new connection, unchanged real 180-second deadline, uncertainty/lockout, late-completion refusal and unchanged one-attempt budget passed. Fifteen checks and isolated schema/container/network cleanup passed.
- [Engine Linux acceptance/publication](https://github.com/flndrn-dev/briven_v2/actions/runs/39037634763): actual save/restore, acknowledged rows/vectors/indexes, independent writes, permissions, parent/client/prepared-work refusal, missing-PID protection, actual removal/interruption/restart/no-resurrection and storage/WAL cleanup passed. Tested config and registry source/platform match.
- [API publication](https://github.com/flndrn-dev/briven-website/actions/runs/39032882302) and [web publication](https://github.com/flndrn-dev/briven-website/actions/runs/39032883676) passed; source/run provenance and deployed manifests verified.

| Image | Immutable selector | Manifest |
|---|---|---|
| API / supervisor | `serverless-20261017-operations-a69f19c-api` | `sha256:43711474966fc8d90636f0b4a4e07bf8059e3d7c5d5c776813f0e149c587091b` |
| Web | `serverless-20261017-operations-a69f19c-web` | `sha256:316ef4c352452e4905d3717e65f8a1b0413781253030697a1099aa96caaa1423` |
| Engine | `operations-07711c126016e0b8c7958a62e5fc95d22a844bd2-39037634763-1` | `sha256:359ca9530ceb35f77ee83d7dea1b1d409ccd9647810cd97df35843ba8ab3bbdc` |

The later website commit adds only the interruption harness/workflow and evidence. Final main/evidence commits are distinct from these accepted application image sources; no new application image is substituted after the matching recovery drill.

## Recovery acceptance

- Trusted final-release staging drill: `e9f629dc-5030-4278-adf8-8cc4ed28e3be`. Three actual failed readings at production cadence; early/observe dispatch refused; concurrent identical signed delivery caused exactly one restart; old child reaped; replacement passed three fresh independent healthy readings; replay and cooldown remained enforced.
- Failed verification on the fixed staging review target: `8236d836-5b08-4126-a3e3-795c262cf303`. A deliberately paused replacement caused durable failure/lockout. The same fixture was repaired and supplied three actual healthy readings.
- Normal owner password verification and review: `623f6cc7-44f3-4540-afb3-17c930afa48a`, recorded 15:03:25 UTC. Review released only this lockout and retained the failed status, cooldown and daily budget. The runner's earlier bounded owner wait safely stopped its child; resuming the same attempt created no second fault or budget reset.
- Owner confirmed activation at action time; the normal authenticated health UI enabled automatic recovery at 15:07:34 UTC. The database retains the real owner actor, matching build and drill. No SQL review/mode bypass was used.
- **Actual deployed automatic-loop attempt:** `ccd49d4a-b9c1-456e-9350-717fefe6f820`. The guarded sole observation child was paused. The deployed supervisor itself collected failures, claimed once, replaced/reaped that child and verified three fresh healthy readings. Final healthy state was observed after **210 seconds**, within five minutes. The supervisor/container stayed running, mode remained automatic and exactly one actual production attempt is retained in its daily budget.
- Twelve actual worker administration requests from anonymous, ordinary project-owner, viewer and foreign identities were refused; authority/history/budget/review state remained unchanged.
- All staging children were reaped; no running recovery claim remains. The normal owner UI shows “Automatic recovery · tested worker only” and the reviewed failure remains in history.

The unchanged limits are three failed readings at least 30 seconds apart, fresh evidence, one active claim, 120-second action/verification bound, 30-minute cooldown and at most two attempts per target per 24 hours. Host pressure remains observation only. Automatic authority includes no Docker, host/database/API restart, arbitrary executable/PID, spending or data removal.

## Saved-copy and restore acceptance

Policy: **keep until explicitly removed; maximum 10 retained manual copies per project; no automatic expiry**. Restored branches have a separate limit of 10. Pending, failed and uncertain operations retain quota until physical removal is confirmed. Parent copies cannot be removed while dependent restored branches remain. Removal leaves durable history/tombstones and cannot be undone by an image rollback.

Implementation includes disjoint short-lived one-target removal grants, durable serialized removal phases, exact identity/activity/dependency checks, smart shutdown, stopped-cache quarantine, confirmed pageserver deletion, exact safekeeper local/remote WAL cleanup and permanent creation/start/wake/credential fences. Website claims/retries preserve immutable identity and recheck current session, role, project and recent verification at dispatch. Ambiguity never releases quota.

The actual deployed drill used only synthetic project `p_01M4V8KYS62GQ49AQQ2NQ8PAQJ`, tenant `25ab618c43b6b9fc102a4dfb111d637e`:

1. Normal authenticated concurrent saves produced one durable copy and a real flushed checkpoint. Changed retry parameters conflicted.
2. The source acknowledged a later write. Only its proven idle main was smart-stopped to honor the unchanged four-compute cap; no client was forced off.
3. Restore retained the saved rows/vectors/indexes and excluded the later source write. Independent restored writes left source unchanged. Parent removal and an actual open restored SQL connection refused removal before destructive intent.
4. After closing the owned connection, normal removal confirmed the restored branch gone and refused new credentials. Normal demand resumed main with its later acknowledged write intact and no restored-only data.
5. The new copy was removed through the normal route; only the new disposable table was dropped. Both new operations are confirmed removed and their quota released.

New disposable IDs: saved `br_01M554N5NS7NB954W5R7QFB32Y`; restored `br_01M5554XP4FCDTTD3C5QCRSMK5`. Physical inspection confirms both new timelines, all three safekeepers' local WAL and remote storage/WAL objects are absent; endpoint/quarantined caches are cleaned; exact permanent tombstones remain.

Native desktop and 390 × 844 review passed: readable retention/limits, typed `REMOVE` gating, Cancel without deletion, progress/retry controls and cleared connection display. Final native public page shows exactly **1/10 saved copies and 1/10 restored branches**, both original resources ready. No database credentials were displayed.

After the owner-confirmed successful recovery check, native Dokploy deployed only the API rollout change. Actual environment is enabled with all eligible projects enrolled. The previously excluded owned project now returns normal history; anonymous/viewer/foreign access remains denied. Public `/api/ready` reports all dependencies ready.

## Preservation and cleanup

- Original **212 rows/vectors** remain exact: `bc3310b7f3de375a5a5e3f6b792fc9d1a474cf6c0680d8ce7b91fd7fbc7f296b`.
- Original schema/index hash remains exact: `13dd5b5b490d10ef946f39f04e7abe3f70bb1db55d10d177da8e8ae0c54a7b7c`.
- Original ready saved copy `br_01M4WVMQGBTCQA7V7968D48YXQ` and restored branch `br_01M4WVMRPS8S3ZZF3XDYZH8NW4`, their mappings and storage remain intact.
- All project metadata, current protected owner deletion timestamps, entity counts, pending customer Auth records and Auth configuration match baseline. Historical deletion flags were never restored.
- Final infrastructure proof passed: all **31 unrelated running services**, existing volumes/mounts/ports and credential hashes preserved. Accepted source/manifests, compiled engine, mounted startup hash, resource socket/cgroup, log caps and lifecycle=false/[] verified. Public rollout retained the exact tested supervisor container.
- All six private journals were backed up/restored and verified around API replacement. Journals/receipts remain private and no secret values are included here.
- All scripted sessions were signed out. The two owned native fixture sessions were revoked through normal own-account list/revoke APIs; unrelated sessions were preserved. Three owned incognito windows closed, credential bridge file deleted and in-memory fixture credentials cleared.
- Initial main-pause helper errors occurred before any pause request (command-line separator and shipped library path). Fixes retained the guards; the successful receipt followed actual client/prepared-work and process-exit checks.
- The first engine CI run failed only the final removed-identity replay expectation after parent removal. It published no image. Commit `07711c126` checks the tombstone before resolving its parent; the complete rerun passed.

## Operating handover and rollback

Verified native target: Briven / production / **briven.tech, 187.77.183.190**. Website Compose `Ju2ZiCJxOw2srPHr0245C`, path `infra/dokploy/compose.serverless-postgres.yml`; engine Compose `tzky4csBhT4ZAGKdhf0R9`, path `infra/private-back-room/compose.yml` plus the existing resource/public-PostgreSQL overlays. Preserve all storage, domains/TLS, profiles and host controls.

Immediate rollback is operational and retains history:

1. Normal owner health control: **Turn off automatic recovery**. Confirm observe mode; retain attempts, reviews, cooldown and daily counts.
2. Disable saved copies or narrow the project rollout in the existing website environment; deploy only the API through native Dokploy. Preserve and restore all private journals if the API is replaced.
3. Keep the removal-compatible engine. Any binary downgrade requires a separately verified drain and continued tombstone/quarantine enforcement. Do not start an older engine against removed identities; a rollback cannot restore removed data.

Final scoped commits/main synchronization, terminal native deployment verification, restoration of the original saved deployment commands and the non-secret target registry complete the repository handover. Temporary service-selection commands prevent unrelated mutable-image pulls during the final OnPush deployment; originals are restored without starting another deployment afterward.

Separate handover gate: credential rotation remains open, including the Resend webhook secret and deployment-trigger URL exposed by native diagnostics. No secret values are stored in this report. These selected capability acceptances do not close that broader gate.
