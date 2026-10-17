# Public rollout and handover — 17 October 2026

**Status: in progress, selected area remains 90%.** Owner selected this area next and added Resend dashboard email-code delivery as the immediate release gate. No final rollout, rollback or credential-rotation completion is claimed.

## Accepted owner evidence

The owner confirms successful recovery-code testing on their own account. Count that flow as passed based on the owner's report; it does not require repetition. Separate untested customer/device flows remain recorded under their own acceptance area.

## Current public readiness

Read-only checks passed for apex, www, sign-in, API readiness, docs, realtime readiness and exact realtime source. Private realtime metrics/operator routes return 404. Existing website target is Briven / production / briven-postgres-website, Compose `Ju2ZiCJxOw2srPHr0245C`, server briven.tech / 187.77.183.190.

Live product source/image identity is now API/supervisor/web **579db5d**, runtime c12c924, realtime b46a55d and engine 743cab2. Product containers are healthy with zero recorded restarts. There are 34 product/retained containers plus one explicitly approved temporary sensitive-action callback. Both protected Mittera project rows retain their current owner-deleted state. Counts remain four users, three organizations, five project rows and zero project files. Saved copies and lifecycle remain disabled; recovery remains healthy in observe mode.

## Immediate gate: Resend

- Owner selected **Briven dashboard first**, then approved the written Resend design and native execution.
- Authenticated native Resend account `flndrn` has verified `briven.tech` and the newly owner-created **Briven** key. Existing Handlr keys/domain/webhook are preserved. No duplicate API key is required.
- Live inspection found the API still using Mittera, without SMTP or Resend credentials. Source had no Resend sender or receiver, so connecting the new key and implementing the receiver are prerequisites to actual email-code login.
- Candidate website source `579db5d40b57f9dc709a6b9e773056382f53cf82` is committed/pushed on `resend-dashboard-20261017`. Approved design and implementation plan live in website `docs/superpowers/`.
- Local verification: **80 tests / 323 assertions / zero failures or skips**, Bun 1.3.14 and disposable PostgreSQL 17.11; four owned databases removed and cluster stopped. API/web typechecks, API bundle, production Webpack web build and focused lint passed. Missing web Bun test types were reproduced and corrected without importing Bun's runtime globals into the browser type environment.
- Exact-source Linux gate [39026386770](https://github.com/flndrn-dev/briven-website/actions/runs/39026386770) passed on clean source `579db5d`: **80 tests / 323 assertions / zero failures or skips**, PostgreSQL 17.11, four disposable databases removed and cleanup confirmed. API/web image publication was dispatched only after this success: [API 39026818514](https://github.com/flndrn-dev/briven-website/actions/runs/39026818514), [web 39026819508](https://github.com/flndrn-dev/briven-website/actions/runs/39026819508), immutable selector `serverless-20261017-579db5d`.
- Owner saved the existing API key and webhook signing secret in the verified website service. All 40 earlier environment entries were preserved; the sender is `Briven <noreply@briven.tech>`. Credentials are masked and absent from reports.
- Owner had created Briven webhook `16cad618-256f-4c7e-bc0f-002df9a4dda1`. Its saved signing secret matches privately. Corrected its URL from the nonmatching website path to **https://api.briven.tech/resend-webhook** and narrowed 23 events to the seven approved delivery outcomes. Resend confirmed the update; webhook remains enabled. Existing Handlr endpoint was preserved. No real delivery is claimed before deployment/login acceptance.
- Native Dokploy deployment **Done** on `579db5d`, now pushed/synchronized on website main. Both publications succeeded and their actual running manifest digests match verified provenance. All 23 changed application files match accepted source by SHA-256. Exact migration hash/timestamp is recorded once; tables exist. All five private journals restored, original container/settings/project/Auth hashes preserved. Eleven public health and webhook security-boundary checks passed.
- Found a release metadata defect: revision labels used the workflow dispatch branch `109845a`, while attestations, actual files and `/info` correctly establish `579db5d`. Workflow label generation now uses checkout identity; verification of corrected labels belongs to the next publication. Existing immutable images were not overwritten. This limitation is explicit in the website acceptance report.
- **Resend functional acceptance passed.** Owner reports successful real inbox/code login. The real `email_otp` message has signed sent/delivered receipts; two native provider replays returned HTTP 200 while the one message, two event rows and three audit effects remained identical. Existing automated OTP/MFA regressions passed; individual native MFA prompts were not independently observed. Website report: `docs/resend-dashboard-acceptance-2026-10-17.md`.

## Remaining handover gates

1. Reconcile all still-open subsystem gates in the production tracker. Generic main CI had pre-existing lint/typecheck failures; do not present a focused acceptance run as full green CI. Verify corrected revision labels on the next image publication.
2. Execute and record a bounded rollback proof with current schema compatibility and exact image selectors. Retain current/previous images and all volumes; the 14 October engine rollback document is a procedure, not an executed final-release proof.
3. Complete current-target credential rotation/revocation and new-works/old-fails proofs, including the newly exposed website/Resend credentials. The prepared 9 October private-control rotation targets retired container/service IDs and API source; it must not be run against the new installations.
4. Remove temporary acceptance access/routes and reconcile owned failed fixture artifacts. Preserve customer data, prior accepted fixtures and referenced volumes/releases.
5. Commit/push final handover evidence and save the accurate service map after all remaining gates pass. Resend-specific source/evidence synchronization is recorded separately.

## Rotation dependency inventory

Read only, values suppressed. Pair dashboard database credentials with API and operations supervisor; Auth database credentials with the API; control identity secret with website API and engine; runtime shared secret with API/runtime/realtime; website object-store credentials with API/website object store; engine object-store credentials with engine/its object store; catalog credentials with engine/catalog. Include previously surfaced engine secrets and the known Dokploy deployment hook in final review. At-rest encryption/master keys and recipient/audit pepper require data-preserving migration/overlap if ever rotated; a blind environment replacement would invalidate existing data or outstanding delivery correlation.

Computer-use policy requires owner entry/submission for new authentication credentials in the UI and action-time confirmation for new privileged access. Prepare exact fields, continuity checks and rollback before that handoff. Never place values in this report or chat.

**17 October environment exposure:** reconnecting the native Dokploy app emitted its revealed environment in a tool result. Values were immediately masked again. Treat the website environment's credentials, including the new Resend key/signing secret, as exposed and include them in the required scoped rotation. Credential values are deliberately omitted here. Preserve encryption-key and audit-pepper dependencies through a reviewed data migration/overlap rather than invalidating stored data by replacing them blindly.

## Evidence locations

Website `.local/resend-dashboard-20261017/` contains private redacted local summary and live preservation baseline. The baseline includes all five project-row hashes and customer Auth/configuration hashes. Sensitive-action temporary fixture and its private journal remain separately documented; back up all five current private journals before API replacement.
