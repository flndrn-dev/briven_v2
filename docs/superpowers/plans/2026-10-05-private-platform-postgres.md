# Private platform PostgreSQL implementation plan

**Goal:** Provision fresh, separate PostgreSQL 17 dashboard and Auth databases privately on the approved KVM2 Briven target.

**Architecture:** The existing website and volumes stay operational. Two new database services have separate volumes and application roles, require TLS on a closed network, and use a private CA. Only the public CA certificate will be mounted into future application containers. Customer computes remain owned by the separate Briven engine.

**Tech stack:** PostgreSQL 17 official image, OpenSSL from the already verified engine image, Docker Compose, Dokploy.

**Spec:** `docs/superpowers/specs/2026-09-28-sprint-3-serverless-cutover-design.md`; owner-approved six-step installation and cutover request on 5 October.

## Constraints and review focus

- Deploy through Dokploy in Briven/production on KVM2; no public database ports or website domain routes.
- New secrets are random and saved only in Dokploy; do not copy old database credentials or import old accounts.
- Dashboard and Auth roles cannot access each other's databases or the engine catalog.
- TLS private keys and the CA signing key never enter application containers or git.
- Persistent volumes cannot reuse the live website volumes; log files are capped.
- Backup restoration and wrong-password/non-TLS rejection must be tested before application cutover.

## Task 1 Provision isolated PostgreSQL services

**Files:** `infra/platform-postgres/compose.yml`, `create-certificates.sh`, `initialize-database.sh`, `check-compose.sh`.
**Interfaces:** Dokploy supplies `BRIVEN_CONTROL_DB_ADMIN_PASSWORD`, `BRIVEN_CONTROL_DB_APP_PASSWORD`, `BRIVEN_AUTH_DB_ADMIN_PASSWORD`, `BRIVEN_AUTH_DB_APP_PASSWORD`. Internal DNS names are `briven-control-db` and `briven-auth-db`; client CA path is `/etc/briven/db-ca/ca.crt`.

- [x] Write/run the closed-network check before the Compose exists; require nonzero exit.
- [x] Add idempotent CA/certificate provisioning, SCRAM application roles and TLS-only host rules.
- [x] Run shell syntax and the closed-network Compose check. Commit/push only these files and this plan; actual Docker rendering is also checked by Dokploy before deployment.
- [ ] Create a separate Dokploy Compose using this repository/branch/path, generate its independent keys and deploy.
- [ ] Verify both services, application role login with hostname-verified TLS, plaintext/wrong-password/cross-database refusal and no published ports.
- [ ] Dump a synthetic probe, restore it into a temporary database, verify the row, and record proof before preparing the website.
