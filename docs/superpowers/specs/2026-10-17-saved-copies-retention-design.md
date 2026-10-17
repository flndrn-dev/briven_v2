# Saved copies and restore — retention, final acceptance and activation

## Selected policy and product boundary

Close the selected 85% → 100% gap for manual saved copies and restore to a new writable branch. Owner selected **keep until explicitly removed, maximum 10 manual copies per project, no automatic expiry** on 17 October. Retain the existing maximum of 10 retained restored branches per project; an explicitly removed branch releases that slot. Failed operations with possible storage still occupy a slot until reconciliation/removal confirms cleanup. This is a manual-copy feature; scheduled backups, point-in-time recovery and cross-region disaster recovery are separate capabilities.

The existing broker captures a flushed WAL position, persists its immutable copy identity before dispatch, and restores to a distinct `rc-` branch. Source and protected copy data remain unchanged by restore. Keep existing verified-user, organization/project admin, current-session and session-bound recent-verification checks. Removal needs the same authority plus explicit resource confirmation. Server state supplies all project, branch, timeline and actor bindings.

## Approaches considered

| Approach | Trade-off |
|---|---|
| Existing-server bounded acceptance — recommended | Uses the owned synthetic fixture and alternates its main/restored compute within the current four-slot capacity. Requires the controlled pauses described below. |
| Dedicated acceptance environment | Adds isolation and infrastructure setup/cost approval, followed by the same public activation checks. |

Use the existing-server approach under native execution. Read-only inspection confirms four active compute scopes, existing saved-copy/restore records on the isolated fixture, and saved copies disabled. Existing engine branch deletion is not exposed; safe end-to-end removal is required to make retained-copy quotas reusable.

## Durable operation and quota behavior

Extend the existing durable operation state with explicit removal-in-progress, removal-failed and removed outcomes, a stable removal request key and a lease/claim token. Preserve create identity/retry keys and historical actor/audit data. Add an additive migration with the matching schema and constraints. Serialize claims under the existing per-project transaction lock with bounded lock/statement timeouts.

An authorized removal commits its exact target and claim before external calls. Fresh authorization and immutable target bindings are rechecked at dispatch. A job being removed cannot be restored, re-created, retried as a creation, or issue new connections. Removed create/retry keys stay tombstoned. Only confirmed completed removal sets `deleted_at`, releases retained quota and removes the operation from active history. A proven engine preflight refusal before any destructive intent/effect releases the removal claim and preserves the ordinary usable job state, recording the refusal separately. Once destructive intent is accepted, failure/timeout keeps the identity frozen for reconciliation/retry; an uncertain response must not say data was removed. Concurrent removal/retry/restore requests cannot dispatch conflicting actions. Old claim tokens cannot complete newer claims.

Ten retained manual copies and ten retained restored branches are separate limits. A deletion request in flight or with unknown storage outcome still counts. Retained failures remain visible and retryable. List responses include removal state and safe operation metadata, never credentials, private engine assertions or lease tokens.

## Engine removal boundary

Add a private reserved-branch removal operation covering only correctly formatted `sc-` and `rc-` names. A short-lived signed grant binds organization, engine project, branch, timeline and removal action; the public generic gateway cannot mint or forward this authority. Require admin/owner authority. Main, ordinary branches, mismatched IDs, service credential scopes and foreign projects fail closed.

Under the engine mutation gate, validate identity, authoritative storage ancestry and applicable compute preconditions before accepting destructive intent. A refused parent/dependency/client check has no destructive effect and leaves the branch usable. After successful preflight, persist a durable removal intent for that exact tuple before effects. Reject branch recreation, credential issuance, compute start and wake for tombstoned/removing identities, including after an engine/API restart. Replays reconcile the same intent. Storage's own deletion guard also rejects any concurrent child dependency. Remove a restored child explicitly before its parent saved copy; there is no cascade.

For a restored branch, verify the exact bound PostgreSQL/controller processes. Active clients, prepared transactions, unknown identity or unavailable activity inspection cause a refusal before shutdown. Request smart shutdown for an idle recognized compute and verify that both bound processes have actually exited. A client arriving during the check is protected by smart shutdown; timeout does not force termination. Quarantine the stopped endpoint definition outside startup enumeration before deleting its timeline, and retain the durable tombstone so replay/startup cannot recreate it.

Delete the exact leaf timeline using existing authenticated pageserver APIs. Their asynchronous acknowledgement is not completion: reconcile until the tenant-valid missing-timeline result confirms completion. Then clean the exact timeline's safekeeper/WAL state using existing authenticated deletion APIs and confirm outcomes. Retain an in-progress/failed receipt across partial cleanup and retry; source/other timelines remain unaffected. Remove only the matching branch mapping and quarantined endpoint cache once cleanup is confirmed. Retain a compact durable tombstone and audit. Bound external work to 120 seconds per dispatch; retries resume the same identity.

Source basis: `pageserver/src/http/routes.rs` deliberately distinguishes missing tenants from missing timelines; `pageserver/src/tenant/timeline/delete.rs` refuses parents with children; safekeeper deletion defaults to local plus remote WAL cleanup. [PostgreSQL smart shutdown](https://www.postgresql.org/docs/17/server-shutdown.html) protects existing sessions and waits for them to finish. [Flushed WAL location](https://www.postgresql.org/docs/17/functions-admin.html#FUNCTIONS-ADMIN-BACKUP) remains the copy checkpoint source.

## API, customer controls and rollout

Extend saved-copy history and add a scoped removal endpoint for existing job IDs. Strict request parsing accepts the stable request key and confirmation only; branch/timeline/actor values come from durable authorized records. Return no-store responses and generic failure categories. Check current project/session/role and recent verification before every sensitive dispatch; same-account verification from a different session is invalid. Keep the generic control gateway's reserved-branch restrictions and legacy overwrite routes unavailable.

Show the selected retention policy, separate retained limits, removal confirmation, progress/failure/retry and completed removal. Removing a restored branch warns about removing that branch's data and identifies the selected branch by its customer-visible name; the source database is preserved. Parent dependency, active connection, capacity and uncertain-outcome refusals are actionable and truthful. Successful removal frees its slot. Match the existing visual style and accessibility; keep private implementation identifiers out of the customer flow.

Keep the global `BRIVEN_SAVED_COPIES_ENABLED` switch and add a project rollout setting, defaulting to no enrolled projects. A one-project allowlist supports actual deployed-route acceptance; an explicit all-projects selection is the final public activation. `/db/info`, lists, creation, restore, removal, retry and connection issuance must enforce the same rollout decision. Existing out-of-scope projects remain unavailable during the bounded trial. Deploy configuration through the verified native Dokploy website target.

## Verification and live fixture bounds

Run real PostgreSQL concurrency/security/removal-state tests and a disposable Linux engine drill before publishing images. Cover retained quota reuse; duplicate and changed request keys; stale leases; scope revocation; same-owner cross-session verification; dependent-parent refusal; active-client/prepared-transaction refusal; graceful process exit; missing/mismatched/foreign identity; failure after each external step; restart reconciliation; pageserver asynchronous deletion; safekeeper cleanup; denied recreation; exact data/vector/index preservation and cleanup. Require zero skips, exact source, bounded logs and artifact checks. Build immutable engine/API/web images in CI only.

Live trial is restricted to synthetic project **`p_01M4V8KYS62GQ49AQQ2NQ8PAQJ`**, engine tenant **`25ab618c43b6b9fc102a4dfb111d637e`**, organization **`org_01M4V8KY00X11YHKYXF8YT3WNC`**, on **briven.tech / 187.77.183.190**. Preserve its 212 existing rows, vector/index/schema hashes, existing saved copy `br_01M4WVMQGBTCQA7V7968D48YXQ`, existing restore `br_01M4WVMRPS8S3ZZF3XDYZH8NW4`, and pending Auth/device fixture. Also preserve every other project/current protected-deletion state, container, setting and volume.

Create one uniquely named disposable acceptance table with synthetic rows/vectors, one new saved copy and one new restored branch, tracked in a private journal. Verify acknowledged contents at save, change source after save, then gracefully pause only this synthetic project's idle main compute to free a slot. Start the new restored compute, verify saved contents and independent writes, close its owned SQL sessions and remove that newly created restored branch through the new confirmed operation. Resume the original main compute and verify its later source change plus absence of restored-only writes. Remove the newly created leaf saved copy and drop only the new acceptance table. Capture current/golden hashes before and after. The four-slot cap and all other computes' policies remain unchanged.

Native customer-control review covers confirmation cancellation, progress/reload/retry, retained limits, scoped restored connections and removal outcomes on these disposable fixture resources. Any user credential/MFA prompts are owner-only. Verify existing copies survive API/engine replacement and retain current source/vector/schema state. A final public activation follows successful actual deployed-route acceptance, cleanup and preserved-state checks. Normal owner review and any required computer-use action-time confirmation occur only after the concrete results are ready.

## Rollout and completion

Revalidate native Dokploy targets: website `Ju2ZiCJxOw2srPHr0245C` and engine `tzky4csBhT4ZAGKdhf0R9`, Briven/production/server briven.tech. Current actual engine container is `briven-clean-engine-20261011-dsjaqo-engine-1`; registry entries are discovery hints and require native revalidation. Preserve startup guards, storage mounts, resource bounds, public routing, TLS and existing lifecycle=false policy. Retain prior images and all data volumes. Back up/restore all five existing journals plus the new shared journal around replacements.

Sequence: implement/test → exact-source Linux acceptance → publish immutable images → native deployment → isolated live copy/restore/removal acceptance → final worker drill/owner activation → explicit public saved-copy activation → actual UI/status/preservation checks → scoped evidence commits/main synchronization. If a new API image is required after worker activation, repeat matching recovery acceptance before reactivation. The immediate rollback disables saved copies and switches recovery to observe through their normal controls while retaining the removal-compatible engine. Any binary downgrade needs a separately verified drain of in-flight operations/assertions and preservation of quarantined endpoints/tombstones; do not blindly deploy an older engine that cannot enforce the new tombstones. Removed data cannot be resurrected by a rollback.

Close this selected gap only when the chosen retention behavior, safe reusable quota, actual save/restore/removal, permission/refusal paths, restart continuity, public activation, cleanup and preservation pass. Report it separately from final credential rotation, scheduled backups and other open platform gates.

**Review status:** Owner approved native execution on 17 October; final activation confirmed at action time. Required acceptance passed; see `../reports/2026-10-17-operations-closure-acceptance.md`.
