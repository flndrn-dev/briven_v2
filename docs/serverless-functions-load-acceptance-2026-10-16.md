# Serverless functions: sustained load acceptance

Completed on 16 October 2026. The selected serverless-functions checklist moves
from **95% to 100%** after isolated acceptance, matching runtime publication and
deployment, a full ten-minute live run and final cleanup all passed.

## Accepted source and corrections

Website source: `c12c9240d65f7d76968031dacaf834c90a8e5bc1`.

The runtime now admits one invocation per project through cold start, execution
and isolate retirement. Separate projects remain concurrent. Result resolvers
and touched-table bookkeeping belong to the owning isolate, so a crash in one
project cannot drain another project's invocation or mix repeated request IDs.
The Deno ready handshake has a bounded five-second startup allowance. The isolate
clears its request context before publishing a completion frame, preventing the
previous turn from clearing the next turn's context.

Focused local verification: **27 tests passed, 611 assertions, zero failures**.
One pre-existing test is skipped; the newer two-project shutdown-drain regression
passes. Runtime typecheck, scoped lint and diff checks passed.

## Disposable Linux acceptance

[Workflow 38787955145, attempt 2](https://github.com/flndrn-dev/briven-website/actions/runs/38787955145/attempts/2)
passed on the exact source above with Bun 1.3.14, Deno 2.8.3 and PostgreSQL 17.

| Check | Accepted result |
| --- | --- |
| Sustained duration | 300,000 ms |
| Invocations | 2,048 requested and completed; zero errors |
| Concurrency | Bursts of 16 across two independent projects |
| Exact acknowledged rows | 1,024 per project; every stored field checked by primary key |
| Project A canonical SHA256 | `939f951af712c440f24eda8438421d59f67954075fce8eebf78224b99ccbc204` |
| Project B canonical SHA256 | `a2a219122d869b86fee93fb388bdc5355eb23a041acef0cc48a9b6011dad234b` |
| Pool size | At most two isolates; both projects cross the 1,000-call retirement boundary |
| Latency p50 / p95 / p99 | 23 / 38 / 116 ms |
| Final reads and cleanup | Passed |

Downloaded artifact `functions-load-summary`, ID `12047194782`, archive digest
`sha256:7acd42b08987397aed28e7213fb488ef8980c72da06c93dff964b95d20e6e3d0`.
The downloaded archive's SHA256 matches that digest, and its single summary
matches the separately downloaded file. The verified summary SHA256 is
`38cdbb868856055e43f96613d39ed93cb3bfa1043fcb52e8c546ba8182b109f2`.
Evidence is retained in the website's ignored `.local/functions-load-20261015/`.

The same run's first attempt never started because GitHub blocked Actions for
account billing/spending. The owner resolved account access before attempt 2.
Earlier actual test failures and their artifacts remain preserved separately:
cold-start timeout, request-context turn boundary, timer duration rounding and
database/JavaScript row-order comparison. None counted as successful acceptance.

## Release and live acceptance

Runtime-only publication succeeded in
[workflow 38905118338](https://github.com/flndrn-dev/briven-website/actions/runs/38905118338)
for tag `serverless-20261015-c12c924` (the tag uses the task's source date).
Registry provenance matches the exact accepted source and publication workflow.
Manifest digest:
`sha256:973e16d218bfd538cacf08e9ccad52eafda042fdf921e78f7f8037327850c8bc`.

The existing Dokploy target Briven / production / briven-postgres-website on
`briven.tech` (`187.77.183.190`, service `Ju2ZiCJxOw2srPHr0245C`) reached
**Done in 3m 50s** at the accepted source. The saved environment adds only
`BRIVEN_RUNTIME_TAG`; all 39 prior saved entries were preserved and checked
after reload. The deployed environment's 42 prior entries, including three
Dokploy-managed entries, are unchanged; the new selector makes 43.

Actual runtime `/info`, pulled image digest and Compose source all match the
accepted release. Bun is 1.3.14 and Deno is 2.8.3. Runtime is healthy, ready and
has zero restarts; boot time is `2026-10-16T14:10:55.406Z`. The container comparison
confirms only runtime changed: all ten other services retain exact container and
image identities, environment hashes, mounts and ports. The existing deployment
command and storage were preserved.

Two initial four-call preflights received four HTTP 200 responses each but failed
an acceptance-helper actor assertion. API-key calls intentionally carry
`key:<key ID>`; the helper had expected the issuing human's ID. The helper now
checks the exact issued key ID for key calls and the owner ID for session calls,
with secret refusal checked separately. Both failed journals and their fixture
rows are retained; cleanup passed and runtime error counters stayed zero. No
runtime or Auth product change was needed for this helper correction.

The corrected four-call preflight passed exact rows, actor, secret refusal,
permission checks, metrics and cleanup, with p95/p99 of 120 ms. The first full
run received all 200 HTTP 200 responses and acknowledged 100 new rows, with
p50/p95/p99 of 62/315/618 ms and clean credential/session cleanup. It failed the
duration gate at 599,999 ms because its timer woke one millisecond early. Its
summary and all acknowledged rows are retained; it is not accepted evidence.
The helper now waits against the monotonic 600,000 ms deadline using the same
loop already proven in Linux CI. A fresh full run on the unchanged runtime passed
with distinct new row IDs and preserved all rows from previous attempts.

| Live acceptance | Accepted result |
| --- | --- |
| Timed duration | 600,001 ms |
| Scheduled traffic | 50 bursts of four calls, 12.2 seconds apart; at most four concurrent and 20 timed invokes/minute |
| Responses | 200/200 HTTP 200; no 429 or 5xx |
| Actual data | 100 new uniquely named writes and 100 reads; every new row and every baseline row checked exactly |
| Latency p50 / p95 / p99 | 62 / 139 / 312 ms; below the 2 s p95 and 5 s p99 gates |
| Actors and permissions | Exact API-key actor; owner-session actor; secret refusal; viewer key/session and foreign-session refusal |
| Runtime success counter | 217 → 419: 200 timed calls plus the initial and final verification probes |
| Runtime errors / crashes / crash-loop events | 0 → 0 for each |
| Restart | Same boot time throughout; no restart |
| Retained fixture/customer data | All baseline rows and both protected Mittera bindings/deletion states preserved |
| Temporary credentials and clients | Keys revoked and verified; sessions signed out and verified; clients closed |

Accepted live evidence: `live-full-2026-10-16T14-35-14.287Z.json`, SHA256
`6245e14328def00ee87a292953a99f6cef56d8e0618d0cb63f382cb8c16e6e1c`.
The running image configuration is
`sha256:06e15dfba45103e2020d489e7f5c7022ab8d122dd1f76df791d17cbba4323485`.

The independent database checkpoint asserts **zero synthetic sessions and zero
active temporary CLI/runtime/function-load keys**. Platform counts remain four
users, three organizations, three projects and zero files. The private fixture
journal was backed up again; retained data and rollback images remain available.
The local disposable PostgreSQL fixture is stopped.

Public apex, www, sign-in, API readiness/info and realtime readiness/info returned
HTTP 200 with the expected API and realtime sources. Public realtime operator
paths returned 404; private monitoring remained available. Recovery remains in
observation mode. No production fault, policy activation or credential rotation
was part of this run.

## Broader CI limitation

[Main CI 38913176500](https://github.com/flndrn-dev/briven-website/actions/runs/38913176500)
passed its test and security-audit jobs, but lint/typecheck failed in the unchanged
web app: an unavailable Next image lint rule, a redundant Boolean call and missing
`bun:test` declarations in three web test files. Build and CLI tarball dependents
were skipped. The web tree is byte-for-byte identical to the prior `389e13f`
release (`git diff --exit-code 389e13f c12c924 -- apps/web` passed). This report
does not claim broad CI success. Focused runtime type/lint/tests, exact-source
Linux acceptance and runtime image publication passed.

The legacy GitHub deployment job also failed for absent Dokploy webhook/API-key
secrets. The existing Dokploy native integration performed the verified rollout;
no new deployment credential was created.

This selected checklist covers function invocation concurrency and sustained
operation. Realtime load, automatic recovery, lifecycle, backups, device/provider
acceptance and billing remain separate work.
