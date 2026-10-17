# Monitoring and incident dashboard acceptance — 17 October 2026

**Status: accepted — Monitoring and incident dashboard 85% → 100% of the selected production checklist.** Real history, stale-source refusal, recovery, native interface, permissions, exact-image Linux acceptance and preservation passed.

## Scope and identity

Continues the owner-approved 12 October aggregate-monitoring contract and the 17 October request to finish current-release acceptance. Host/compute resource monitoring remains observation only; the separately accepted observation-worker recovery remains automatic. Public manual saved copies stay enabled, database lifecycle stays false/[], and no resource limits or customer state change.

Accepted running API, supervisor and web application source: `a69f19c6507d9bcbd036574f39e1b4806f611bb0`. API manifest `sha256:43711474966fc8d90636f0b4a4e07bf8059e3d7c5d5c776813f0e149c587091b`; web manifest `sha256:316ef4c352452e4905d3717e65f8a1b0413781253030697a1099aa96caaa1423`. Engine remains accepted `07711c126016e0b8c7958a62e5fc95d22a844bd2`.

Installed producer and service/timer hashes match the reviewed tracked sources. Producer SHA-256: `c5754fc0c51f37c6f490e487f49759d8d2338364788def882b9d6da8868b91d5`. Fixed parent accounting is `brivencompute.slice`, three CPUs, 8 GiB memory, zero swap and 1024 tasks. The unprivileged sandboxed timer runs every 30 seconds. Existing scraper/observation cadence is preserved. No Docker inventory/stat polling or host build was added.

## Real history, formulas and incidents

- Preflight at 16:25:53 UTC: each compute metric has 7,083 durable records and at least 7,078 independent source timestamps, beginning 12 October. Each host metric has 8,383 records and 8,380 independent timestamps, beginning 11 October.
- Exactly three compute metrics (CPU, memory, tasks) and three host metrics (CPU, memory, disk) have separate targets and learned baselines. No fictitious compute disk metric or parent/child double counting.
- Current production collector percentages match independently queried Prometheus accounting using the exact three-CPU, 8-GiB and 1024-task denominators. Producer and scrape freshness are checked independently; invalid/out-of-range metrics remain unknown.
- The normal owner dashboard exposes the corresponding values, one-decimal rounding, target labels, healthy-history counts and learned medians. Existing 27 recovered incidents remain visible with original evidence and timestamps. No production pressure or synthetic incident was injected.

## Actual stale-producer acceptance

Only the fixed `briven-compute-metrics.timer` was stopped after its source/unit/state guards passed. Engine, databases, observer, exporter and Prometheus continued running.

- At 16:32:17 UTC the real producer sample was 134 seconds old while exporter up and current scrapes remained fresh. All three production compute readings and dashboard projections were unknown; all three host readings stayed fresh and normal.
- At 16:32:47 UTC source age reached 164 seconds. The actual production observer had durably recorded null values, null source timestamps and unknown assessments for all three compute metrics.
- Native Chrome showed all compute cards as “— / No fresh reading” while host CPU/disk/memory remained populated. Existing incident history was retained. The automatic observation worker remained healthy throughout.
- The guarded `finally` block restored the producer and active/enabled timer. Successful pause duration was 195.9 seconds, below the six-minute bound.
- By 16:37:51 UTC every host and compute metric had **four independent fresh production samples after restoration**. All six current assessments were normal and baselines recovered from retained history. No fabricated timestamps or direct production history writes were used.

## Exact-image Linux acceptance

[Linux run 39051292529](https://github.com/flndrn-dev/briven-website/actions/runs/39051292529) succeeded from harness `05453e010061d93c5e6db598d92b4aeb73eb119d`: **31 tests, 140 assertions, zero failures/skips**, PostgreSQL 17.11, plus the four exact-producer filesystem tests. The actual accepted production API image was pulled by immutable manifest, and its config `sha256:0b2d585c24bff652e30bbc883dcf1abdb1123786e23eb7fc0db1d47c9f821fd5` matches the running API baseline. Production collectors/store/overview and actual React component were exercised; only the new test and acceptance runner were mounted. No application image was rebuilt. Owned schema, container and socket cleanup passed and the downloaded artifact was independently checked against the successful run identity and running image config.

## Interface and permission acceptance

- Complete focused local gate: **31 tests, 140 assertions, zero failures/skips**, PostgreSQL 17.11. Disposable schemas were empty afterward, PostgreSQL stopped and its owned directory removed. Includes collectors, statistical baseline, actual SQL incident transitions/persistence/retention/concurrency/rollback, shared worker/API cadence and actual React component behavior.
- Four actual rendered React component tests pass locally, 26 assertions: six scoped labels/rounding; stale compute with fresh host; failed API hiding old values followed by recovery; warming-up and acknowledged-but-open incident state until genuine resolution.
- Native desktop review passed for stale and recovered states. Native 390 × 844 review passed for readable target labels, metric values, baselines and normal-state explanations. Device emulation was disabled and DevTools closed afterward. Console entries visible during review were extension `contentscript.js` warnings; no browser protection was weakened.
- Eight deployed requests from anonymous, ordinary project-owner, viewer and foreign-owner identities refused monitoring reads and incident acknowledgments with 401/403. Incident/acknowledgment history stayed identical. All three scripted fixture sessions were normally signed out; no new credentials or owner session was created.
- Four producer filesystem regressions pass locally and cover actual aggregate values/time, changed/missing bounds, malformed/impossible readings, previous-success replacement and symlink refusal.

## Preservation, cleanup and handover

Before/after proofs preserve **all 35 running containers** exactly, including images, credentials/environment hashes, mount/port bindings and health/restart metadata. Saved Compose environment is unchanged. Producer/unit hashes, active/enabled timer and actual cgroup bounds match baseline. Customer project/job/Auth metadata hashes are identical, including current owner deletion states and existing saved copies. Recovery attempts, review, approvals, mode changes, drill and target authority hashes are identical; no additional automatic recovery attempt was consumed.

Application product source is unchanged by this task. Only acceptance tests/workflow and evidence are added, so the matching recovery build/drill remains valid and no new application image publication is required. Native Dokploy remains the deployment control plane; temporary API-only selection during the final main push prevents unrelated mutable-image pulls, and the original saved command is restored afterward. Private journals are preserved around any actual API replacement. Only scoped source/evidence is committed; pre-existing unrelated edits remain intact.

Private redacted receipts: website `.local/monitoring-acceptance-20261017/`. Historical failed receipts are retained. The first stale helper omitted the required fetch dependency in its independent query and safely restored the timer; the corrected drill passed. The initial component fixture used JSDOM's hidden-document default, which correctly suppressed refresh; a visible-document fixture passed without a product change. Initial Linux database initialization used only a custom Unix socket, while the [official image entrypoint](https://github.com/docker-library/postgres/blob/master/docker-entrypoint.sh) initializes through its default socket; retaining both sockets fixes the isolated fixture without network exposure. Subsequent harness fixes preserve its explicit Unix fixture user/locale and use the built-in Bun test command instead of the repository's Turbo script. A stalled run was canceled; bounded test/attach deadlines and owned cleanup now prevent indefinite waiting. Fresh-only snapshot guards refused observations taken during the intentionally stale interval. None of these harness corrections alters monitoring policy.

The authoritative monitoring row is now 100%. Carrying forward the already accepted worker-recovery and manual-copy scores, the same equally weighted 20-area estimate is **91.50%**, up from 90.75%; these percentages are engineering estimates, not a whole-platform certification.

This report covers the selected monitoring/history/incident-dashboard acceptance. It does not certify broader billing, customer-device Auth, lifecycle activation, credential rotation or final public rollback/handover gates.
