# Briven build continuation — 5 October 2026

**Goal:** Restore the owner's dashboard password login, then resume the September 30 private engine deployment plan.

**Architecture:** Keep the existing website and its control database operational. Repair only the explicitly named owner account using the deployed authentication library. Continue the private engine separately on the website's verified Dokploy server, with no public storage ports or website engine cutover.

**Tech stack:** Better Auth, PostgreSQL, Rust, GitHub Actions, Docker Compose, Dokploy.

**Specs:** `2026-09-27-briven-website-engine-deploy-design.md`, `2026-09-28-sprint-3-serverless-cutover-design.md`, and `2026-09-29-customer-database-home-design.md`.

## Constraints

- Owner account: `flndrn@hotmail.com`. Never record its password, password hash, or session token in repository files or reports.
- Preserve unrelated local edits and live customer data.
- Keep the public website on its existing stack while proving the private engine.
- Use the existing Briven Dokploy project and the server attached to website compose `LykAyuz6qInYZe37wIMLp`; validate that identity before deployment.
- Private engine: persistent storage, three disk-synced safekeepers, PostgreSQL catalog, and private MinIO.
- Do not expose engine administrator credentials. Leave customer connections unavailable until the TLS proxy actually works.
- Public engine cutover remains the separate final step of the original plan.

## Tasks

- [x] Inspect the owner record and deployed superadmin allowlist. Verify password login, session, owner profile, admin authorization, and dashboard access. The existing account works; no password reset was needed.
- [x] Correct the PostgreSQL submodule URLs while retaining the exact pinned commits. Confirm a clean GitHub checkout can fetch them.
- [x] Build and publish a new immutable engine image. GitHub Actions run `37305069988` succeeded for `sprint3-engine-20261005-1`.
- [ ] Verify the published image's runtime files and executable dependencies before installing it. Manifest inspection confirms Linux amd64; local runtime inspection cannot run because the Docker daemon is not running.
- [ ] Validate the Dokploy target and create the separate private engine Compose service with dedicated volumes and securely generated settings.
- [ ] Complete the original private-room startup, catalog backup/rollback, proxy, isolation, and restart checks in order. Record evidence and stop on unresolved failures.

## Review focus

- An owner reset must never update another account or broaden the superadmin allowlist.
- Authentication proof must check protected routes, not only a successful sign-in response.
- Submodule source changes must not change the pinned PostgreSQL revisions.
- An image tag is deployable only after its build succeeds and runtime dependencies are verified.
- A healthy existing website is not proof that the new private engine works.

## Verified progress on 5 October

The owner login, session lookup, `/v1/me`, and `/v1/admin/overview` all returned HTTP 200. The owner profile is an administrator. Dashboard counts now handle PostgreSQL query results correctly. Password fields have SHOW/HIDE eye controls, including independent controls for confirmation fields.

The live `/admin/login` page was also checked in Brave with a dummy password: SHOW revealed the existing value and changed to HIDE with an eye-off icon; HIDE restored a masked field and SHOW with an eye icon. No sign-in was submitted during this visibility check.

Website commit `7098e1b` preserves the live PostgreSQL control database in the checked-in deployment configuration. Dokploy deployment `UJUOuZ3tn5oAZEGIYrbfL` completed successfully; `/ready` reports ready. This repair does not migrate customer Auth or customer databases. The same repair is on the website rebuild branch as `62475e7`.

The engine image is `ghcr.io/flndrn-dev/briven-engine:sprint3-engine-20261005-1`; its index digest is `sha256:6cdb74d55027d0ae47d4c4137efbe4bdfb07332a24912691201bf1b4a4d277cc`. The private engine has not been installed or connected to the public website.

GitHub now retains only `briven-website` and `briven_v2`. Local Git mirrors preserve the three retired repositories; see [the repository map](../../briven-repositories.md).

The new Handlr Auth request is tracked separately in [the Auth readiness audit](../research/2026-10-05-briven-auth-readiness.md). Its unanswered policy choices do not authorize changing existing tenants' login requirements.
