# Saved copies retention implementation plan

> Execute natively under the owner's 17 October approval. Keep unrelated working-tree edits intact.

**Goal:** Ship retained manual copies with safe removal, reusable limits, real restore acceptance and controlled public activation.
**Architecture:** The website owns authenticated durable operation claims; a narrowly scoped engine grant authorizes removal of one reserved leaf branch. Engine intent/tombstones survive retries and restarts. Storage deletion and stopped-endpoint cleanup must be confirmed before the website releases quota.
**Tech Stack:** Existing Rust/Axum/control-plane clients, Bun/TypeScript/Hono, PostgreSQL/Drizzle, Next.js, Linux CI, native Dokploy.
**Spec:** `../specs/2026-10-17-saved-copies-retention-design.md`, owner-approved 17 October.

## Global constraints

- Keep until explicit removal; maximum 10 retained manual copies and 10 retained restored branches per project. No automatic expiry or cascade.
- Preserve original/previous saved data, all existing volumes/settings, protected current deletion states and the pending Auth fixture.
- Only exact `sc-`/`rc-` branch identities; private signed grant, current admin/owner session and recent session-bound verification. No arbitrary path/command or public generic deletion authority.
- No forced compute/client termination; use proven idle processes, smart shutdown and actual exit checks. Unknown identity/activity refuses removal.
- CI builds only; bounded 10m × 3 logs; no Docker inventory/stat polling. Keep capacity at four active compute scopes and lifecycle disabled.

## Review focus

- A storage timeout after accepted removal must not restore ordinary ready state or release quota.
- A blocked preflight must leave the existing branch usable; distinguish it from an accepted uncertain removal.
- Old image/startup or a stale creation request must not resurrect a quarantined/deleted branch.
- Copy removal racing with restore/connection issuance must preserve dependent data.
- A reused request key or expired claim must not select another timeline or complete a newer operation.

## Task 1 — Scoped engine identity and durable removal intent

**Files:** root `control_plane/src/briven_control_api/{auth,saved_copy}.rs`; create `saved_copy_removal.rs`; wire in `control_plane/src/bin/briven_control_api.rs`; targeted `local_env.rs` mapping persistence; tests in those units.
**Interfaces:** `RemovalScope { project_id, branch, timeline_id, action }`; principal scope is disjoint from customer/platform/runtime authority. `saved_copy_removal::remove(State<AppState>, HeaderMap, Path<(String,String)>, Json<RemoveBranch>) -> ApiResult<Json<RemovalReceipt>>`. Receipt outcomes are confirmed removed or explicit preflight blocked; external uncertainty is unavailable/retryable, never removed. Durable intent keys include the exact tenant/branch/timeline tuple and phase.

- [x] Add failing Rust tests for valid/mismatched/foreign/main/ordinary/uppercase/malformed identity; admin/owner-only scope; removal token denial on customer/service routes; mixed scopes; unknown grant fields.
- [x] Implement strict signed scope validation under existing 60-second issuer/audience rules, keeping ordinary credentials unchanged. Wire only the private reserved-removal route.
- [x] Add failing journal tests for atomic persistence/reload, exact replay, changed tuple refusal, preflight rejection with no intent, partial accepted intent and removed tombstone rejection of recreation/start/credentials/wake.
- [x] Implement intent storage, exact mapping removal and atomic persistence. Guard branch creation and all endpoint resolution paths before effects. Verify scope/persistence tests and commit only this deliverable.

## Task 2 — Actual engine storage and idle compute removal

**Files:** new `saved_copy_removal.rs`, existing `lifecycle_process.rs`/`lifecycle_runtime.rs` only shared safe inspection interfaces; create root `tests/acceptance/saved-copies/` fixture/drill and a bounded Linux workflow.
**Consumes:** validated scope and durable intent from Task 1. **Produces:** `removed` only after storage and associated stopped endpoint/mapping are confirmed cleaned; blocked receipts before accepted intent; retry preserves phase after accepted intent.

- [x] Add failing preflight tests for active clients, prepared transactions, parent dependencies, unknown/recycled PID, missing mapping with unknown storage identity and another endpoint in the same tenant.
- [x] Reuse bound process/activity inspection; check leaf ancestry; persist intent; request smart shutdown for an idle restored compute; require actual process absence; quarantine only that endpoint definition outside startup enumeration.
- [x] Use existing pageserver deletion API until tenant-valid missing timeline confirms completion, then exact safekeeper local/remote WAL cleanup. Bound dispatch to 120 seconds; preserve intent after any ambiguous step. Remove exact mapping/cache only after cleanup; retain tombstone.
- [x] Run disposable real Linux engine acceptance: acknowledged rows/vectors/indexes, copy/restore independent writes, parent refusal, active connection/transaction refusal, real leaf deletion, duplicate removal, actual interruption/restart at removal phases, no resurrection, storage/WAL cleanup and exact source preservation. Record cleanup; commit.

## Task 3 — Durable website removal and rollout authority

**Files:** website migration `0072_saved_copy_removal.sql` and journal/schema; `services/saved-copies-{policy,store,executor}.ts`, `saved-copies.ts`; create `saved-copy-removal.ts`/integration tests; `engine-project.ts`; `env.ts`, `routes/db.ts` and focused tests; existing private Compose.
**Interfaces:** extend `SavedJob.status` with `removing | removal_failed | removed` plus stable removal key/prior-state/actor metadata. `claimRemoval({projectId,id,actor,requestKey})`, `finishRemoval(claim,outcome)`, and `removeSavedCopy(...)` preserve immutable creation identity. `savedCopiesEnabled(projectId)` applies global switch plus strict JSON rollout (`[]` none, named project IDs, or explicit `['*']` all).

- [x] Add failing real PostgreSQL tests for duplicate/changed removal requests, ten retained copies, quota released only after confirmed removal, blocked-preflight restoration of usable state, timeout/interruption retaining quota, stale claim rejection, restore/delete races, removed create-key refusal and retry after reload.
- [x] Add the forward migration, durable bounded claims and typed removal service. Record actual actor and safe operation/outcome audit; current project/session/role/recent verification must be checked at dispatch. No credentials/grants/claim tokens in receipts/audits.
- [x] Mint only one-target removal grants after authorization; strict engine response validation distinguishes preflight blocked from accepted uncertainty. Generic gateway remains unable to forward DELETE or reserved branch credentials.
- [x] Wire no-store removal and retry routes with strict confirmation/body schema, existing recent-MFA/rate limits and rollout checks shared by all saved-copy capabilities/routes.
- [x] Run zero-skip PostgreSQL/HTTP/scope suites; API/web types and focused lint; verify Compose parses with all log caps and the API-only rollout setting. Commit.

## Task 4 — Customer removal flow and release acceptance

**Files:** website `apps/web/src/app/(dashboard)/dashboard/projects/[id]/snapshots/saved-copies-controls.tsx` and focused component tests; create `scripts/run-saved-copies-acceptance.mjs` and matching Linux workflow; root/website acceptance reports and private resumable helpers.

- [x] Add failing component assertions for typed confirmation/cancel, removal-in-progress/reload/retry, branch-data warning, parent/client refusal, same-request resumption and 10-copy retention text. Match the existing style.
- [x] Add controls and truthful status/limits. Connection display hides on mutation/expiry; only the selected named resource is confirmed. No private implementation fields enter the UI.
- [x] Run local and exact-source Linux API/real PostgreSQL/UI/engine acceptance, types/build/lint and cleanup. Publish immutable engine/API/web images only after the matching gate; verify manifests/provenance/files and native target identity.
- [x] Back up all five existing journals and the new shared journal. Deploy engine and website through existing native Dokploy services, retaining images/volumes/startup/resource/TLS configuration; restore journals and verify hashes/health.
- [x] Enroll only the named synthetic fixture. Run the exact approved alternating-compute live save/restore/removal drill at the four-slot cap; preserve the old 212-row dataset and old copy/restore. Review actual desktop/narrow UI and deployed permission/retry outcomes; clean only new disposable resources.
- [x] Coordinate final recovery acceptance/owner activation under its separate plan. Activate saved copies publicly only after passing fixture/cleanup checks and final confirmation. Verify actual capabilities, UI, storage state and public readiness. Commit/push scoped evidence/main sync and update non-secret target registry; mark this gap closed only on passing evidence.

## Execution record — 17 October

All required acceptance and activation gates passed. See `../reports/2026-10-17-operations-closure-acceptance.md` for exact sources, Linux runs, live operation/review/drill IDs, preservation and cleanup. Resumable native acceptance helpers stayed under private `.local/operations-closure-20261017/`; the additional actual-executor Linux harness is `tests/acceptance/worker-interruption/`. These concrete paths replace illustrative runner names above. The accepted application image remains a69f19c; later website changes are test/evidence only.
