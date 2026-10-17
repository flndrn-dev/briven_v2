# Monitoring and incident dashboard final acceptance

> Continue natively under the approved 12 October compute-monitoring design and the owner's 17 October selection of this final-acceptance gap. Preserve later owner-approved worker recovery and saved-copy activation.

**Goal:** Close the selected monitoring/history/staleness/dashboard checklist on the actual current release.
**Architecture:** Existing fixed cgroup producer → existing node exporter/Prometheus → production collector and durable PostgreSQL history → normal owner dashboard. Acceptance observes this path; no new monitoring service, authority or synthetic production metrics.
**Tech Stack:** Python/systemd, Prometheus, Bun/TypeScript, PostgreSQL, existing React admin interface, isolated Linux CI.
**Spec:** `../specs/2026-10-12-compute-group-monitoring-design.md`. Its historical recovery-observe instruction is superseded by the owner's accepted 17 October automatic-worker release; preserve that exact automatic mode/build/drill.

## Global constraints

- Fixed `brivencompute.slice`, 8 GiB, three CPUs, 1024 tasks, zero swap. Producer every 30 seconds; no increased scrape/observation cadence or Docker polling.
- Source and scrape age at most 120 seconds; future tolerance 30 seconds. Missing/stale/invalid readings become unknown, never healthy zero.
- Host CPU/memory/disk and compute CPU/memory/tasks remain separate targets/baselines. No double counting parent/child cgroups.
- Preserve all running containers/images, volumes, ports, environment, resource bounds, customer data, owner deletion states, incidents, recovery history and six private journals. Lifecycle stays false/[]; public copies and tested automatic recovery stay enabled.
- Pause only `briven-compute-metrics.timer`, for at most six minutes, with guarded source/unit identity and unconditional restoration. Never pause/restart engine, observer, database, exporter or Prometheus.
- No new credentials, spend, infrastructure, public endpoint, production load/stress or injected production history. Keep all receipts private and redact secrets.

## Review focus

- Fresh scrape timestamps must not refresh an old producer timestamp.
- Stale compute data must not erase healthy host readings or resolve an existing incident.
- Duplicate collection must not manufacture independent history or recovery samples.
- UI must show unknown after API failure/old source, correct target/metric/value and truthful learning/incident state.
- Recovery automatic authority must remain separate from host/compute observation and unchanged by this acceptance.

## Task 1 — Freeze identity and inspect actual history

Files: private website `.local/monitoring-acceptance-20261017/`; existing host producer/units, collector/store/overview and admin components.

- [x] Record exact running images/source, producer/unit hashes, resource bounds and current operating mode. Capture infrastructure and customer metadata hashes without exposing values.
- [x] Read actual producer and Prometheus values; compare percentage denominators with fixed cgroup bounds and exact production query expressions.
- [x] Confirm six correctly scoped metric histories, distinct source timestamps, normal cadence, baseline count and current overview projection. Record at least three newly observed production cycles across this acceptance.

## Task 2 — Exact-release isolated regression acceptance

Files: existing `operations/{baseline,collector,compute-collector,store.integration,observe-compute.integration}.test.ts`; root `compute-resource-metrics.test.py`; add only a bounded acceptance runner/workflow where necessary.

- [x] Run actual producer filesystem regressions and operations tests with disposable PostgreSQL, zero skips and owned cleanup.
- [x] Exercise incident pressure/acknowledgment/repeated-sample/unknown/three-reading recovery, separate targets, stale/future/missing/malformed/ambiguous values, transaction rollback and retained history.
- [x] Run the selected tests in isolated Linux using the current immutable production API image or exact matching source; verify image/source identity and cleanup. No application image publication when product source is unchanged.
- [x] Regress and correct any reproduced contract violation before proceeding. A product-image change requires matching release gates/native rollout and preserves or re-establishes the accepted recovery activation contract.

## Task 3 — Real staleness and native dashboard acceptance

Interfaces: fixed phases `preflight`, `stale`, `recovery`, `postflight`; no caller-provided PID/unit/project/SQL target.

- [x] Verify fresh producer and durable state first. Guard and stop only the fixed timer; preserve its original active/enabled state and allow any running oneshot to finish.
- [x] Wait real wall-clock age beyond 120 seconds while exporter/Prometheus/observer remain live. Confirm all compute collector/overview values unknown, healthy host state retained, no fabricated history/incident recovery and automatic worker still healthy.
- [x] Review the actual owner's dashboard in the stale interval: correct targets, unknown compute values, fresh host values and honest recovery controls. Owner handles credentials if needed.
- [x] Restore producer and timer in `finally`, then verify fresh source/scrapes and at least three independently timed durable production readings. Review recovered desktop and narrow interface, exact labels/value rounding/baselines/incidents.
- [x] Confirm unauthorized admin reads/acknowledgments are refused without history changes, using existing owned fixture identities and normal sign-out cleanup.

## Task 4 — Evidence and handover

- [x] Verify all infrastructure/customer/operating hashes remain exact, timer restored, no owned sessions/processes and public dependencies ready.
- [x] Commit/push only scoped plan/test/evidence changes and synchronize both main branches. If native OnPush deployment runs, verify terminal status and unchanged accepted images; preserve private journals around any API replacement. Restore original saved deployment commands after the final trigger.
- [x] Update the authoritative monitoring row to 100% with evidence, carry forward the already accepted worker-recovery/manual-copy scores, and compute the same 20-area estimate (91.50% if all other scores remain unchanged). Preserve unrelated edits and all historical entries.
- [x] Report the selected gap closed only when every required acceptance and cleanup result passes.

## Completion evidence

Accepted current release: API/web/supervisor a69f19c. Local and exact-image Linux gate 39051292529 pass 31 tests / 140 assertions / zero skips, plus four producer regressions. Actual 195.9-second metrics-timer pause, durable unknown state, native desktop/390×844 review, eight permission refusals, four independently timed restored cycles per metric and complete infrastructure/customer/recovery preservation passed. No product source or application image change was needed. See `../../monitoring-acceptance-2026-10-17.md`.
