# Guided project Agent critical-flow acceptance — 16 October 2026

Status: **accepted — Guided project Agent100% of the approved checklist.**
Local/Linux regression, live critical flows, operating checks and main synchronization passed.

Selected area: Guided project Agent, previously95%. Completion means the approved
critical-flow checklist, not whole-application production certification.
Design and native execution were approved by the owner on16October2026.

## Scope and source

- Website acceptance source: `032b7d9da421298afc50b8f401adf320c5d1b32e`.
- Approved design: website `docs/superpowers/specs/2026-10-16-guided-agent-acceptance-design.md`.
- Implementation plan: website `docs/superpowers/plans/2026-10-16-guided-agent-acceptance.md`.
- Product behavior passed without a production code change. Added reproducible
  Linux/disposable PostgreSQL acceptance, composed HTTP confirmation/audit
  regression and five tests of the actual rendered Agent component. DOM testing
  packages are development dependencies only; production dependency versions
  remain unchanged. Unrelated pnpm metadata/transitive changes were excluded.
- All12 selected deployed API product files match the tested source bytes.
  Deployed API remains `063e7f53910eae9c6bbb1b9fcda8e8e065dd1bec` and web product
  source remains `79233845638d362f558d81cf7eb12c6fb81ba32b`. Runtimec12/realtimeb46
  were preserved. No new image publication or runtime rollout was needed.

## Disposable regression

Local PostgreSQL17.11/Bun1.3.14: **47 tests,278 assertions,0failures,0skips**
across8files; disposable schemas returned to baseline and PostgreSQL stopped.

Final Linux run: [38969449358](https://github.com/flndrn-dev/briven-website/actions/runs/38969449358), **SUCCESS**, exact source032b7d9:
47tests/278assertions/0skips, PostgreSQL17.11/Bun1.3.14, API types/build, focused
Agent/component lint and cleanup all passed. Artifact12075854549 was downloaded
and verified: archive SHA256
`63823e4c07cdf5d88d0c19e78cecaf31b7cf9e84be8b8243a99524d184acc839`;
summary SHA256 `353a822d7608edd561629e3c46004869d949433e6f9a52d9bfbaaf05d7824cf7`.

Coverage includes scope and service choices, quota/availability/permission/expiry
rechecks, exact and concurrent one-use confirmation, cancellation before and after
claim, blocked model/database/service work, partial/uncertain outcomes, persisted
claims and history isolation/content deletion. The composed HTTP regression
proves wrong hash/concurrent/replayed confirmation cannot repeat a write, and
deleting its conversation preserves its action outcome.

Five new rendered-component tests control delayed HTTP responses while using the
actual `AccountAssistant`, not a replacement component: close, unmount, workspace
switch and New chat each discard a late startup response against the original
organization using keepalive; closing during a message aborts its signal and
refuses late visible output. These supplement server/database race coverage.

Earlier Linux runs38966329352,38966953327 and38967455789 failed in disposable
PostgreSQL startup before Agent execution. Retained diagnostics identified the
official image bootstrap client's socket/port assumptions; the fixture now
shares its default Unix socket directory and sets PGPORT55449, with no published
port. Run38967689324 passed the earlier42test/244assertion gate, API types/build,
focused lint and cleanup on c4b9af4. Those earlier results are not substituted for
the final47test source. No failed or skipped run is counted as final acceptance.

## Actual deployed critical flow

The normal public app was exercised in a separate Firefox private session as the
retained synthetic owner; no customer account performed Agent mutations.

- Fresh opening created no thread. An ambiguous creation request asked scope
  before a project write. A successful real-provider new-application recommendation
  matched the stated purpose; safe fallback text was explicitly excluded as proof.
  Cancelling the setup left no project or actionable proposal.
- Database-only intake showed the exact name/purpose and Auth/storage/functions
  alloff. Wrong-hash confirmation returned400 and created nothing. One native
  confirmation completed; replay returned400 without a duplicate.
- Exactly one new project, reservation and engine binding were independently
  verified: project `p_01M538Y3YNWM5NXY57Z3M0XE4G`, engine
  `907b9e7fd657de8d3522a1e9772b30c4`, in the synthetic owner's organization.
  Database/mainready, optional servicesoff, correct Setup link and normal
  session-verified, credential-private, authorized read-only `SELECT 1` passed.
- One real-provider advice turn produced an answer without a mutation proposal.
  Done saved it to completed history. Opening was read-only; Archive, Archived,
  Restore and Delete worked. Native date was `16 oct 2026, Fri`; server timestamps
  persisted. Deletion confirmation correctly explained project/audit retention.
- The mobile UI was visibly waiting when the separately budgeted recommendation
  was cancelled. Composer reset immediately; independent server checks found a
  discarded thread with0messages and no proposal. No late output reappeared.
- Close button, Escape and page navigation discarded unfinished content and
  reopened fresh. Escape returned keyboard focus to Agent. Native desktop420px
  maximum and360×800responsive checks showed readable choices/history, visible
  composer and red rounded Cancel immediately before Send, without observed
  horizontal overflow.
- Five live synthetic security refusals passed: viewer project write, viewer
  thread/action access and foreign-owner thread/action access. Revocation, expiry,
  quota exhaustion and forced partial/failure races were tested off production.

There were14message submissions including the refused viewer message. Durable
reservations peaked at4perrollingminute, below the10limit. Three provider-backed
turns were allocated: recommendation≤1call, advice≤5calls, cancelled
recommendation≤1call, for a worst-case7calls under the8call ceiling. Two successful
provider results and a pending cancellation were observed; exact upstream call
count was not instrumented. Browser/proof actions were serialized, with at most
two owned HTTP requests during cancellation. No provider-backed turn or ambiguous
creation was automatically retried; no production failure injection occurred.

## Cleanup and preservation

- All8owned threads are discarded/deleted with no conversational messages,
  title/intake content, active run claims or pending/running actions. This includes
  deletion of the creation chat after verification: the separate successful action
  and exactly one creation audit remain, along with its project and engine storage.
- Normal test sign-out and an independent checkpoint confirm **0synthetic active
  sessions and0temporarykeys**. SQL clients were closed; local PostgreSQL stopped;
  CI disposable schemas were cleaned. Credential handoff was loopback-only, its
  clipboard cleared/tab closed and helper stopped; credentials were never printed.
- Exact final platform counts: users4,organizations3,projects5,project_files0.
  The expected project delta is+1. All pre-existing synthetic thread/action/message/
  reservation rows and pre-existing project identities/bindings/deletion states
  were compared against the task baseline and preserved.
- All34running container IDs/images/environment/mount/port hashes and deployment
  settings matched the pre-acceptance baseline. API boot14Oct11:02:57.191UTC,
  runtime16Oct14:10:55.406UTC and realtime16Oct20:22:56.588UTC were unchanged.
  Recovery remainsobserve; lifecycle/savedcopies remain disabled.
- Both protected Mittera projects remain owner-soft-deleted, with exact organization/
  engine bindings and deletion/update timestamps unchanged (12Oct15:44:14.952UTC
  and16Oct18:56:12.956UTC). The historical journal flag was never restored.
- Public apex/www/signin/APIready/docs/realtimeready/info returned200; public
  realtime operator metrics/stats remained404. No customer data, payment/SMS
  settings, old installation or rollback image was changed.
- Original fixture journal was privately refreshed in its existing backup. New
  task journal is backed up outside the API container at
  `/root/briven-acceptance-20261016-guided-agent/fixture-journal.json` (directory0700,
  file0600). Keep this retained project attributable for later acceptance tasks.

Private native observations, immutable baseline, stage receipts, source hashes,
CI artifacts and failure diagnostics live under website
`.local/guided-agent-acceptance-20261016/`.

## Completion and remaining platform scope

Website local main/origin/main are clean and synchronized at
`60abed48b1bf5be986ca26a5964a6889efc3790d`; task branch is retained. Native Git
integration through the existing Dokploy website service reached **Done8s** at
`e03a28677ed1c2271605b3936f7dab169da5ccd6`, then **Done6s** for final documentation
at60abed4. Independent post-deployment proof confirms the actual checkout60abed4,
all34container identities/settings unchanged, and identical deployment variables
and Compose definition. The existing non-secret target record was updated to
this verified checkout/acceptance receipt, preserving image fields and other
repository mappings. Exact tested acceptance source remains032b7d9; later commits
only record its evidence. Whole scoped diff was self-reviewed; no independent
review or broad pre-existing CI pass is claimed for this test-only change.

Only the authoritative Guided project Agent row moved95%→100%. Other tracker rows
remain unchanged; the20core-area engineering estimate is now**88.5%** (optional
GCS excluded), recalculated from the same20scores. It is not measured test coverage.
Account AI streaming/provider breadth, public SDK distribution and all other
launch gaps retain their separate status.
