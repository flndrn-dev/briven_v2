# Account AI assistant acceptance — 17 October 2026

Status: **accepted — Account AI assistant 100% of the selected checklist.**

The owner selected final provider, streaming and permission acceptance and explicitly
chose streamed progress followed by a validated final result. Local/Linux regression,
real-provider flows, native confirmation/cancellation, preservation and cleanup passed.
The governing documents are the owner-approved 6 October complete-AI design and website
`docs/superpowers/specs/2026-10-17-account-ai-acceptance-design.md` and matching plan.

## Scope and implementation

Exact accepted website source: `f068d91f1cfd2736cba350c5ecb4e765da66258c`.

Account chat previously returned JSON only. Added a session-only SSE message route
beside that compatible JSON endpoint, using the same body/rate/actor/organization/
thread checks. Fixed progress stages describe actual thinking, scoped tool checks
and answer preparation. Only a committed, validated answer/proposal becomes the
terminal result; failures produce a generic terminal error. Provider tokens and
unvalidated tool arguments are not streamed into the UI.

The existing five-step lookup ceiling,30second model deadlines, current permission
checks, durable cancellation/version fences and exact one-use write confirmations
remain. SSE has10second heartbeat comments and a170second deadline. The browser
has a175second deadline,128KiB frame and1MiB response bounds, split UTF8/CRLF support,
strict progress/result checks, EOF/error/duplicate rejection and reader cleanup.
Existing component fences guard progress and final state after close, unmount,
organization changes, New chat or Cancel. Known-length oversized requests have an
explicit 413 response; invalid inputs stop before provider dispatch.

No provider/model/configuration, dependency, action authority, project or service
was added. Existing pricing/payment/SMS and operating-activation settings stay intact.

## Reproduction, regression and Linux gate

The original baseline passed47 tests / 278 assertions against disposable PostgreSQL 17.11.
Four new composed-route tests first failed404 because streaming was absent. Six new
actual-component tests first failed because progress/stream cancellation was absent.
These failures preceded the implementation. Private receipts retain the attempts.
An intermediate fixture expected413without a Content-Length; that body was safely
rejected400. The fixture now exercises the declared-length413boundary explicitly.

Final local and [Linux38977942981](https://github.com/flndrn-dev/briven-website/actions/runs/38977942981):
**90 tests, 480 assertions, 0 failures, 0 skips** across 13 files. Existing Guided Agent,
action/confirmation, lifecycle and onboarding regressions are included.

Coverage includes progress before blocked provider completion, committed final
results, scoped tools, malformed/provider/revoked/viewer failures without success,
no private provider-error leakage, real TCP disconnect abort, durable claim release,
split UTF8/CRLF/heartbeats, bounded/malformed/partial/duplicate results, pending
proposal delivery and confirmation binding. The real React component was rendered
under JSDOM; no replacement component supplied its behavior.

Linux was **SUCCESS** at exactf068d91 with a clean tracked worktree, Bun 1.3.14,
Node 22 and PostgreSQL 17.11. Frozen installation, API/web types, API build, production
web build, scoped lint, disposable schema cleanup and owned PostgreSQL container
removal passed. Local PostgreSQL was stopped after cleanup; local builds also passed.
Artifact12079024831 was downloaded and independently matched GitHub's digest:

- Archive SHA256 `60dc5ed70e7bcc693e78fc6fd59f4022ed207db36b19b2b8e46f6177ab3bb8ea`.
- Summary SHA256 `57156e2061ee7af1954d311ef535fcffbe1aea57037c8406b81c87b5ea365e4c`.

The whole scoped diff and failed attempts were self-reviewed. Broader pre-existing
CI and an independent reviewer are not claimed by this focused gate.

## Publication and native rollout

Both linux/amd64images published successfully from exactf068d91 with immutable tag
`serverless-20261017-f068d91`. Registry provenance independently matches source and run.

| Image | Publication | Manifest digest | Actual image ID |
| --- | --- | --- | --- |
| API | [38978768181](https://github.com/flndrn-dev/briven-website/actions/runs/38978768181) | `sha256:15ee960b230417895354186e0a8877d7b1f94d97cad4fa6e9b39eb0c73f7a2bd` | `sha256:51adc06fad201c8cd901eaadd87d3df2c02bcb217c77f4f0d7cf4921eda15161` |
| Web | [38978769428](https://github.com/flndrn-dev/briven-website/actions/runs/38978769428) | `sha256:37964b55cebd4c5486a5dcc30be16f3ff4eac239c4ec0eb5d991f6eccf244a8d` | `sha256:563da0cbc1d4be8eed86d609e9dd789a83bfae932dd2b2f7561484d43978f9ef` |

Existing native Dokploy target was revalidated: Briven/production/
briven-postgres-website, Compose`Ju2ZiCJxOw2srPHr0245C`, registered server
briven.tech187.77.183.190, repository/main and existing Compose path. The release
workflow serializes these builds. The compatible API deployed first, reaching
**Done2m41s**, enabling backend checks while web built. The subsequent native web
deployment reached **Done2m14s** at the same source. SSH performed bounded proof and
owned fixture work only; deployment used Dokploy's native controls/integration.

Only the API selector changed in phase1 and only web in phase2. Each40entry saved
environment was privately compared, saved, reloaded and masked:38original values
remain exact across both phases. The supervisor shares the API image and was
necessarily replaced; its product source and recoveryobservepolicy remain unchanged.
No saved build-identity variable was introduced; image build metadata owns identity.

All 16 selected deployed product-file hashes match the accepted source, covering
the actual component/client, conversation/routes/tools/actions/lifecycle/provider,
supervisor and preserved Database AI hook. Actual Bun 1.3.14, Nodev22.23.3,
Next build ID`Kiy5O8mLzrhNcLSBXK9Yp`, API boot16Oct23:21:48.360UTC verified.
Runtimec12, realtimeb46 and engine743 remain unchanged.

## Actual provider, permissions and native operating acceptance

Only the retained synthetic identities/orgA/project
`p_01M4V8KYS62GQ49AQQ2NQ8PAQJ` were selected. No customer account performed mutations.

- Fifteen live refusals passed: anonymous/viewer/foreign thread, message and discard
  access; bearer identity on the session-only route; empty/oversized/forged actor-org
  inputs; viewer database creation. The deterministic viewer creation request had
  no successful result or provider allocation. Disposable tests independently prove
  guards precede provider work and cover revocation, expiry and race conditions.
- Real owner and viewer turns streamed thinking/checking/preparing and a validated
  final answer. Both used the scoped project tool and returned the exact two permitted
  project names/IDs, without a write proposal or protected customer identity.
  Observed durations3.104s/3.910s, response bytes1084/1083. SSE content/no-store/
  no-buffer headers, framing and one terminal result were checked.
- Native Firefox private used ordinary synthetic sign-in. Its session survived the
  API replacement. After a fresh UI load, a real provider request produced the exact
  display-name proposal: original projectA →`Account AI Acceptance 20261017`.
  Native progress was visible; Network Monitor showed200/SSE,6769ms. The exact
  project ID/name and Needs your confirmation were reviewed. Independent backend
  proof established that the preview had made no project change.
- Wrong-hash, viewer and foreign-owner confirmations were refused. One native
  confirmation completed the rename. Independent backend verification found one
  succeeded action and one separate action audit; replay returned400. The original
  display name was restored through the normal authorized project PATCH API.
  Its attributable updated timestamp and the two business changes remain audited;
  no backdating or direct metadata repair was performed.
- At360×800, completed content, composer and controls were readable/reachable.
  A fresh final read-only request visibly entered Thinking/Answering. Native Cancel
  acknowledged cancellation and reset to Ready/empty composer. Before cleanup,
  independent proof found the new thread alreadydiscarded, with0activeclaims,
  messages and proposals. No late output reappeared. API outbound HTTPS connections
  were independently0after cancellation. Real TCP abort and component races also
  passed in the isolated gate.

Exactly 4 provider turns were reserved before dispatch, each bounded by the existing
5 lookup ceiling: **20 upstream calls worst-case**, not an instrumented billing count.
Durable reservations exist outside the replaceable API container too. Minimum
quiet interval between completed turns was71.196s; reserved worst-case peak5per
rolling minute, below10. No provider-backed turn was retried automatically. One
provider turn ran at a time, with bounded confirmation/navigation/proof operations.
The quiet guard refused one early viewer attempt before reserving or dispatching;
the remaining8seconds were honored before its single actual submission.

## Cleanup and exact preservation

- All 6 owned new threads are discarded/deleted with no conversational messages,
  title/intake content, active claim or pending/running proposal. The separate
  succeeded rename action and its one action audit remain. All pre-existing
  synthetic conversation/action/history/onboarding rows were preserved exactly.
- Normal browser/helper sign-out and independent checkpoint confirm **0synthetic
  sessions and0temporaryCLI/runtimekeys**. SQL clients/readers closed; credential
  handoff clipboard cleared, temporary tab closed and helper stopped. Responsive
  mode was closed; owner sessions were untouched.
- Final visible schema/index hash
  `13dd5b5b490d10ef946f39f04e7abe3f70bb1db55d10d177da8e8ae0c54a7b7c` and all 212
  `gap_acceptance_data` rows, hash
  `bc3310b7f3de375a5a5e3f6b792fc9d1a474cf6c0680d8ce7b91fd7fbc7f296b`, match baseline.
  All 5 project/engine bindings and deployment/file/environment/schedule metadata
  remain intact. Original display name restored; only its attributable timestamp
  changed. The prior Guided Agent project remains preserved. No new project,
  branch, optional service or generated SQL/code execution occurred.
- Both protected Mittera projects remain owner-soft-deleted at exact organization/
  engine bindings and deletion/update times12Oct15:44:14.952UTC and16Oct18:56:12.956UTC.
  Historical fixture deletion flags were never applied to customer state.
- Platform counts remain4users/3organizations/5projects/0files. All 34 containers
  run; the 31 outside API/web/shared-supervisor match original IDs/images/settings/
  mounts/ports. Replaced services preserve stable environment, mounts and ports.
  Compose definition and all saved variables except the two selectors match baseline.
  Subsequent post-release checks match all 34 container identities and every variable.
- Pre-existing boot history retained; one accepted API boot added. Recoveryobserve
  is healthy onf068; lifecycle and savedcopies stay disabled. Public apex/www/signin/
  APIready/docs/realtimeready/info returned200; public realtime metrics/stats404.
- Four exact fixture journals were backed up before API replacement, restored with
  private modes and semantic equality checks, and refreshed after acceptance at
  `/root/briven-acceptance-20261017-account-ai/` (directory0700/files0600).
  Private receipts, artifact, hashes, budget and helpers remain under website
  `.local/account-ai-acceptance-20261017/`. Credentials/drafts are not published.

## Completion

Only Account AI assistant moves90%→100%. The same20core-area engineering estimate
is **89.25%**, previously88.75%, optionalGCS excluded. This is a delivery estimate;
other launch areas retain their separate scores.

Website local main/origin/main are clean and synchronized at final documentation
commit `2d27bdb2f332de5c9c81fb0bcc8fb7b9df19e366`. The accepted task branch remains
at f068d91. Native documentation deployment reached **Done5s** at2d27bdb; independent
proof verifies that checkout, all34post-release container identities/settings and
every saved deployment variable unchanged. The accepted API/web images remain f068.
The existing nonsecret Dokploy target record now contains these verified source,
image/digest and acceptance receipts; all other repository mappings and prior
backend/Agent/Database AI/realtime fields are preserved. Root report commits are
scoped to this file; owner tracker, mixed handoff and unrelated root edits remain
preserved locally.
