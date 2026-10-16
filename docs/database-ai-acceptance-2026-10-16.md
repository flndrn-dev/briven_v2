# Database AI assistant acceptance — 16–17 October 2026

Status: **accepted — Database AI assistant100% of the selected checklist.**

The selected final-backend and operating acceptance is complete. Execution began
16October and finished17October local date. Other launch areas retain their scores.
The existing owner-approved6October AI design/provider plan and website
`docs/superpowers/plans/2026-10-16-database-ai-acceptance.md` govern this work.

## Fix and verification

Exact accepted website source: `a7d5a83307eb880a665db38e89e48cd377eb80f2`.
The only product change is `apps/web/src/lib/use-ai-stream.ts`: mounted-generation
guards refuse stale responses; reset, replacement and unmount abort opening
requests and cancel/release active readers. The public hook API/forms are retained.
No API/provider/configuration change or new project/service was required.

The original baseline passed35tests/76assertions. Nine tests render the actual
React hook under JSDOM with deferred fetch/read/body promises. Before the fix,
8failed; after it all9passed with31assertions. They cover delayed200/403/503after
reset, replacement streams, unmount during fetch/read, delayed error bodies,
partial EOF and exact whitespace/Unicode/escaping. No replacement hook supplies
the behavior under test.

Final local and Linux: **45tests/109assertions/0failures/0skips**, plus a separately
isolated composed-HTTP child: **3tests/65assertions/0failures/0skips**. The45total
includes its wrapper; these are separate receipts. Real Hono routes, auth/role/body
guards, provider adapter, draft services and SSE are composed with a fake upstream.
They prove unauthorized/invalid input stops before upstream work, developer draft
access, admin-only Studio planning and unsafe-plan normalization. Existing real
socket/provider tests cover aborts, bounded timeout, malformed/truncated/oversized
output and provider errors without production fault injection.

Final Linux [38972365641](https://github.com/flndrn-dev/briven-website/actions/runs/38972365641)
was **SUCCESS**, clean exacta7d5a83, Bun1.3.14/Node22/frozen pnpm. API types,
focused web types, scoped lint and production Webpack build passed. Local production
build also passed. Downloaded artifact12076604428 matched GitHub's digest:

- Archive SHA256 `97ac3a1e0e9870fc1c7befac87329b1507a2768e600d1e8272c56bc72ea9867c`.
- Summary SHA256 `016dad15ae5f71892dbef6f80a50b715e5eab410f5061993f09d94ec6da9e875`.

Earlier Linux38971948944 passed behavior/API types but failed the focused harness
because it omitted the base project's `.next` exclusion. The harness was aligned
with that existing configuration; product/build type checking stayed enabled.
An initial sandbox loopback-listen failure passed with required native permission.
Failed runs are retained and do not count as acceptance. Broader pre-existing CI
is outside this gate. The full scoped diff and receipts were self-reviewed;
no independent review is claimed.

## Publication and live deployment

Web-only [publication38973160080](https://github.com/flndrn-dev/briven-website/actions/runs/38973160080)
was **SUCCESS** in15m34s, exacta7d5a83,linux/amd64. Registry provenance independently
matches the source and workflow. No API/runtime/realtime image was rebuilt.

- Image `ghcr.io/flndrn-dev/briven-serverless-web:serverless-20261016-a7d5a83`.
- Manifest `sha256:040c5a64db8f9f229e3aaf154ec5ee02341ccca67debe752e6137332a313f9e5`.
- Running image ID `sha256:69d6004a25552b6a59e90691f91a9cc514b941dca27301071a243d4f5edde1cd`.
- Actual Next build ID `uG6EUtPDYftHQa6tOZnQ6`.

The existing native Dokploy target was revalidated: Briven/production/
briven-postgres-website, Compose`Ju2ZiCJxOw2srPHr0245C`, registered server
briven.tech187.77.183.190, repository/main and existing Compose path. Only
`BRIVEN_WEB_TAG` changed. All39other saved values across40entries were privately
compared, saved, reloaded and verified before masking. Main was fast-forwarded and
pushed; native Git integration reached **Done2m51s** at a7d5a83. SSH performed
bounded proofs/synthetic acceptance only, never deployment.

All12 selected deployed product-file hashes match the accepted source, including
the hook/parser and AI/Studio/provider services. The footer reflects unchanged
API063; actual image/build/source evidence identifies the new web release.
API063/runtimec12/realtimeb46/engine743 remain unchanged.

## Final-backend and browser acceptance

Only retained synthetic project `p_01M4V8KYS62GQ49AQQ2NQ8PAQJ` was selected,
using ordinary synthetic owner/viewer/foreign logins.

- All21unauthorized requests across six generation endpoints and Studio planning
  were refused. All13empty/oversized inputs returned400. Zero provider allocations
  accompanied these checks; composed-route tests prove refusal before dispatch.
- Three real JSON requests returned nonempty schema/function/explanation, model
  identifiers and elapsed receipts. TypeScript drafts parsed with the requested
  Briven interfaces; explanation covered vector ordering and bounded results.
- Three real SSE requests had correct content/cache/no-buffer headers, exact
  decoded tokens, one done and no error/partial completion. Schema66tokens/227chars,
  function78/253 and explanation293/1219 passed. JSON durations1.565/16.905/14.260s;
  SSE durations7.636/8.591/17.436s.
- One real Studio request returned a structured proposed table/columns. No apply
  request occurred. Generated code/SQL/plans were never executed against live data.
- Native Firefox private checked all three deployed forms on desktop and360×800:
  notices, inputs and lower actions were readable/reachable. The eighth request
  visibly showed streaming. Navigation to function cancelled its POST, independently
  shown as **NS_BINDING_ABORTED** in Network Monitor. The first accessibility link
  action did not navigate; clicking the visible function tab completed navigation
  while that same request was pending. No extra generation was submitted.
- Reopened schema showed0promptcharacters and no stale draft/stream. Normal browser
  sign-out passed; responsive mode closed. Handoff clipboard cleared/tab closed,
  loopback helper stopped; owner sessions were untouched.
- Independent API network observation found **0established outbound HTTPS
  connections** after cancellation. No test provider connection remained active.

Exactly8owned provider-backed requests were durably reserved, peak6perrollingminute,
under the10limit; no automatic retry. Requests were sequential apart from bounded
navigation/proof work, at most2owned HTTP requests. This counts application requests;
provider internal billing calls were not instrumented.

## Preservation and cleanup

Before drafts, an authorized read-only SQL transaction captured visible schema/
index definitions and every customer-owned row. Initial customer SELECT correctly
refused reserved `_briven_` tables; the helper respected that boundary without
changing privileges. Final schema/user-row comparisons were exact:

- 212`gap_acceptance_data` rows, hash
  `bc3310b7f3de375a5a5e3f6b792fc9d1a474cf6c0680d8ce7b91fd7fbc7f296b`.
- Schema/index hash
  `13dd5b5b490d10ef946f39f04e7abe3f70bb1db55d10d177da8e8ae0c54a7b7c`.
- Exact projects, deployment/file/environment/schedule metadata and service boot
  history match baseline; all5bindings, including the prior Agent fixture, preserved.
- Both Mittera bindings remain owner-soft-deleted at exact deletion/update times
  12Oct15:44:14.952UTC and16Oct18:56:12.956UTC. No historical flag was restored.
- Post-verification checkpoint confirms **0synthetic sessions/temporaryCLI-runtime
  keys**. SQL clients/readers closed. Counts remain users4/orgs3/projects5/files0.
- All34containers running; only web changed. The other33IDs/images/environment/
  mount/port hashes match exactly. Web mounts/ports, Compose definition and all
  variables except the web tag match baseline. API/runtime/realtime boots remain
  14Oct11:02:57.191UTC,16Oct14:10:55.406UTC,16Oct20:22:56.588UTC.
- Recoveryobserve; lifecycle/savedcopies disabled. Public apex/www/signin/APIready/
  docs/realtimeready/info200; public realtime operator metrics/stats404. Payment/SMS
  settings unchanged.
- Original fixture and final task journals privately backed up outside API at
  `/root/briven-acceptance-20261016-database-ai/` (0700/0600). Private receipts,
  native observations, artifact, hashes and helpers remain in website
  `.local/database-ai-acceptance-20261016/`. Credentials/drafts are not published.

## Completion

Only Database AI assistant moves95%→100%. The same20core-area engineering estimate
is now**88.75%**, previously88.5%, optionalGCS excluded. This is a delivery estimate,
not measured coverage or whole-application production certification. Account AI,
public SDK distribution and other launch gaps retain their separate status.

Website local main/origin/main are clean and synchronized at final documentation
commit `9f0f050da3a3350ba12d55bfd882794659f7de93`; the accepted task branch remains
at a7d5a83. Native Git deployment reached **Done6s** at9f0f050. Independent final
proof verifies that checkout and all34post-acceptance container identities/settings
unchanged. Public readiness passed again. The existing nonsecret Dokploy target
record now contains the verified checkout, accepted web image/digest and acceptance
run, preserving every other mapping and existing backend/Agent/realtime fields.
Root report is committed/pushed separately; owner tracker, mixed handoff and all
unrelated local changes are preserved. The accepted product image remains a7d5a83.
