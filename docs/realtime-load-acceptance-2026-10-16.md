# Realtime load and reconnect acceptance — 16 October 2026

**Status: Complete —100% of the selected realtime checklist.** Exact-source Linux
acceptance, publication, native deployment, ten-minute live acceptance and final
cleanup/preservation passed. Other launch gaps remain unchanged. Public SDK
distribution remains open as described below.

## Scope and implementation

The owner approved native execution of the realtime design and implementation
plan in the website repository, `docs/superpowers/{specs,plans}/2026-10-16-realtime-load*`.
The accepted software source is `b46a55d81123cbbb8fb0d813edee8b555b4e8dba`.

- The JavaScript client owns one connection generation and retry timer, avoids
  duplicate registration, cancels late connections and forwards scoped errors.
- PostgreSQL listeners resnapshot after reconnect and cancel late setup.
- Snapshot work is serial per exact project/function/serialized arguments,
  coalesces pending notifications and permits at most32 concurrent query groups.
  Each project occupies at most one group slot; another project can progress.
- Every subscriber is authorized independently for each dispatch. Only a pending
  query result is shared; authorization decisions are never cached.
- A second read after new LISTEN/trigger attachment closes the discovery gap.
  Failed listener registration returns a scoped error and releases its resources.
- A stalled TCP consumer is closed at a1MiB outgoing buffer bound. The client
  reconnects for current state; healthy consumers continue independently.
- The realtime image pins Bun1.3.14, matching acceptance.

These are query snapshots: intermediate burst/outage states may be coalesced.
Every paced committed state and the final burst state were checked exactly;
snapshots never moved backwards. This adds no replay log or exactly-once event
delivery guarantee.

## Linux acceptance

[Workflow38958146250](https://github.com/flndrn-dev/briven-website/actions/runs/38958146250)
passed on the exact source above with Bun1.3.14, Deno2.8.3 and PostgreSQL17.
It used actual realtime, WebSockets, Deno isolates and four disposable project
databases. Control-plane key/limit responses used fixture adapters; query data
and commit notifications came from real SQL. No production credentials or
production failure injection were used.

| Check | Accepted result |
| --- | --- |
| Tier admissions and next-subscription refusal | Free100 / Pro1000 / Team10000; all three refusals exact |
| Sustained allowance | 10000 subscriptions across50 sockets,300000ms |
| Paced revisions | 30; every healthy subscription converged to each |
| Commit-to-snapshot p50/p95/p99 | 557 /617 /2556ms; gates<2000ms p95,<5000ms p99 |
| Realtime peak RSS | 187416576 bytes, approximately179MiB; gate<2GiB |
| Fixture process / Deno processes peak RSS | 94543872 /272318464 bytes, measured separately |
| Query invocations / per-subscriber key checks | 59 /467136 |
| Runtime failures | 0 |
| Owned reconnect waves / listener recovery | 3 /1 |
| Burst commits | 100; exact latest-state convergence |
| Final data / resource cleanup | Exact state hashes; zero subscriptions, channels or pending dispatcher groups |

The same workflow passed8 SDK reconnect regressions, the installable artifact
test (which repeats those8 tests through its exported entry), and27 realtime
registry/scheduler/socket/real-SQL tests. Type and lint checks passed, including
React/Vue/Svelte types. The socket suite included an actual paused TCP receiver,
argument isolation, viewer/foreign/revoked-key refusal, pending-close fences and
listener-error cleanup. Uncommitted and rolled-back state was refused.

Artifact12071318725 archive SHA256:
`81bb6f5fba3d3b203c287eef712bb7fc069cd9ec1977c30436d1996a5743d7a2`.
Load-summary SHA256:
`f10ab343cabacb727ec29fc451eb619f5f0740bdb19017ab4a4a49dc939ac1da`.
Downloaded copies are retained in website `.local/realtime-load-20261016/`.

## Failure history

Fail-first tests reproduced duplicate/late SDK registrations, abandoned retries,
ignored error frames, missed listener-gap state, a connection surviving shutdown,
the initial read/LISTEN gap, backwards snapshot delivery, unbounded stalled-peer
retention and tenant slot starvation. Review also reproduced and corrected an
introduced listener-error cleanup regression before release.

- CI38956529688 stopped before capacity testing because a package test assumed
  filesystem directory order. Sorting before the exact-content comparison fixed it.
- CI38957047876 passed the full capacity test onf44eea2. It is retained as earlier
  evidence; the source above is the final accepted release.
- CI38957808058 was deliberately cancelled after the additional error-path fix
  superseded it. No cancelled run is counted as acceptance.
- Native baseline helpers initially assumed a build environment field and the
  wrong working-directory depth. Actual `/info` source and Compose labels resolved
  those fixture assumptions; production settings were untouched.
- The pinned Bun client lacks the newer pause() API; the stalled-receiver test
  uses an actual RFC6455 connection with its TCP reader paused.

## SDK delivery

`briven-client-0.0.1.tgz` was built, inspected and installed offline in a disposable
consumer. Its exported JavaScript entry loads in Node, bundles for browsers and
passes reconnect tests. The archive contains only the manifest and four approved
entry/declaration/map files; it has no runtime workspace dependency.

Archive SHA256:
`871e5e96f31c7bea9c71d7eb015586b0eb4fe3a209fe5da21b4efdb00a3ff386`.

Public npm lookup and authenticated collaborator lookup both returned404 for
`@briven/client`. Public package distribution remains open; no npm publication
was performed or claimed. The existing HTTP invoke suite has two unrelated
failures (non-JSON and network exceptions); that path was unchanged. Earlier
unrelated broad web CI failures remain separate from this accepted realtime job.

## Deployment, live acceptance and preservation

[Publication38959188791](https://github.com/flndrn-dev/briven-website/actions/runs/38959188791)
passed for realtime only, `linux/amd64`, tag `serverless-20261016-b46a55d`.
The workflow definition came from main80a7f87; its explicit checkout and verified
image provenance identify the acceptedb46a55d source.

- Manifest: `sha256:25d8197d1646b4b78a2da605c8311a6a18e2905cd9ed293e172ecfd215eacbf9`.
- Running image: `sha256:003b0231fd0832c245e4b6c29f1b0817c3ee84956935cc7942847c2f2ffad696`.
- Native existing Dokploy target: Briven/production, briven-postgres-website,
  service `Ju2ZiCJxOw2srPHr0245C`, registered briven.tech/187.77.183.190,
  repository main, `infra/dokploy/compose.serverless-postgres.yml`.
- Only `BRIVEN_REALTIME_TAG` changed. All39 other saved entries were identical
  after reload (40 total); runtime remains`serverless-20261015-c12c924`.
- Native Git deployment reached **Done3m49s** atb46a55d. Actual source, pulled
  digest, Bun1.3.14, health/readiness and restart0 were verified.
- All33 other running container identities/images/environment/mounts/ports and
  the Compose definition were unchanged. Existing command, volumes and routing
  were preserved. Public apex/www/signin/API/docs/realtime checks passed; public
  realtime metrics/operator paths remained404.

An initial preflight stopped before creating keys or writing any marker because
the historical fixture journal's second protected project deletion flag was
outdated. Read-only audit showed a normal organization-owner `project.delete`
at18:56:12.959UTC (deletion timestamp18:56:12.956), before the first execution
commit at19:18:50 and this rollout. Both organization/engine bindings matched.
The owner's current soft deletion was preserved, the historical journal retained,
and a separate task baseline pinned the exact current protected state and audit
evidence. No customer project field was changed by this work.

The corrected preflight passed:8 owned subscriptions,2 revisions at least12201ms
apart, p50/p95/p99 94/114/114ms,4 viewer/foreign refusals, exact210 baseline query
rows plus1 retained synthetic marker row, zero error/crash metric growth and
complete key/session/socket cleanup. A quiet window longer than60s preceded the
full run to preserve the aggregate rolling write budget across attempts.

### Full live acceptance

The exact accepted packed SDK connected over system-trusted public WSS. The
declared fixture query returns id/title, secret-refusal and actor fields; every
baseline query value was compared exactly. Only the retained synthetic project
was exercised. No production process/database fault was injected and no ambiguous
write was retried.

| Check | Accepted result |
| --- | --- |
| Monotonic duration | 600000ms |
| Marker revisions | 50/50, minimum spacing12200ms |
| Owned subscriptions | At most8;6 healthy subscriptions continued after key revocation |
| Received snapshots | 394, including reconnect/current-state refreshes |
| Owned SDK reconnect waves | 3; fresh current state received before another write |
| Expected security refusals | 6: viewer/foreign HTTP and WSS checks plus2 revoked subscriptions |
| Snapshot p50/p95/p99 | 81 /111 /2271ms |
| Runner direct HTTP concurrency / rolling writes | 1 /16 per minute; both within approved bounds |
| Exact query data | 211 baseline rows preserved plus1 retained synthetic marker row at revision49 |
| Dispatch/query/runtime errors, crashes and crash loops | All0 before and after |
| Service boots | API, runtime and realtime unchanged throughout acceptance |
| Cleanup | Zero owned subscriptions, channels, queued/running groups, active temporary keys and synthetic sessions |

Full live summary SHA256:
`87b7e28483c2d4f4b85dd3727275f505ce87b13a98b9f3513979a01e045ed38c`.
Baseline query hash:
`6ca2b4e0c109ddd2d0012f8ecd4c074713c301edc012c09a1c55e2f066b52347`.
The preflight and full run each retained one owned synthetic marker row. All
earlier fixture rows, including earlier failed/accepted attempt evidence, remain.

### Final preservation and source synchronization

- Website local main and origin/main are synchronized and clean at documentation
  commit`96383eb575384a1d2f1efeb81514fd788664bf63`. Native Dokploy reached
  **Done6s** for that commit. The actual realtime software remains the accepted
  b46a55d image; its boot is still`2026-10-16T20:22:56.588Z`, healthy/restart0.
- Final independent proof retained all33 other container identities and exact
  image/environment/mount/port hashes, the other deployment variables and Compose
  definition. Runtimec12 and its boot`2026-10-16T14:10:55.406Z` remain unchanged;
  API boot remains`2026-10-14T11:02:57.191Z`.
- Both protected project bindings and their pinned owner deletion/update
  timestamps match exactly. The historical journal is unchanged and privately
  backed up. No customer field or storage was changed.
- Final public readiness and private realtime-route protection passed. Synthetic
  keys/sessions are revoked, recovery remains observe, and the dedicated local
  PostgreSQL fixture is stopped. Existing lifecycle/saved-copy policy is preserved.
- Only the authoritative realtime row in the owner tracker moves95%→100%.
  The website plan records acceptance; this report is committed separately in
  the root repository. Pre-existing unrelated root edits are preserved.
