# Private customer proxy implementation plan

**Goal:** Connect customers to the correct Briven compute through the inherited Neon TLS proxy, then prove isolation, persistence and restoration before website cutover.

**Architecture:** The existing Neon proxy uses its control-plane backend. New private callback routes authorize a separate proxy-only secret, map registered endpoint IDs to ready catalog projects, and return only that compute's unexpired customer-role SCRAM verifier. Customer URIs include a PostgreSQL startup endpoint hint. The private proxy has no public ports; website routing is integrated only after the server proof.

**Tech stack:** Rust/Axum, tokio-postgres, inherited Neon proxy, PostgreSQL/pgvector, Dokploy Compose.

**Spec:** `docs/superpowers/specs/2026-09-28-sprint-3-serverless-cutover-design.md`; owner's authorized six-step deployment request.

## Constraints and review focus

- No legacy accounts or rows are imported. Preserve old website volumes for rollback.
- Proxy callbacks reject website identity tokens, missing/wrong proxy secrets and administrator roles.
- A credential for one tenant cannot authenticate to another endpoint, including a stopped compute.
- PostgreSQL password verifiers, passwords and tokens never enter logs.
- Disable proxy role and compute caches initially so revocation and restart cannot reuse stale state.
- Require client TLS and verify the private CA and hostname in the server proof.

## Task 1: Proxy control-plane adapter

**Files:** `control_plane/src/briven_control_api/proxy_auth.rs`, `catalog.rs`, `customer_credential.rs`, `control_plane/src/bin/briven_control_api.rs`.
**Interfaces:** `GET /proxy/get_endpoint_access_control?endpointish=…&role=…` returns `role_secret`; `GET /proxy/wake_compute?endpointish=…` returns `address` and Neon metrics `aux`. Only `BRIVEN_PROXY_CONTROL_SECRET` authorizes these routes. Customer connection URI carries `options=endpoint%3D<registered-id>`.

- [x] Test missing/wrong/shared-identity tokens, tenant-role binding and endpoint hints.
- [x] Implement ready-project lookup, parameterized SCRAM lookup and bounded stopped-compute resume under the lifecycle lock.
- [x] Run focused Rust tests/typecheck, commit and publish a versioned engine image.

## Task 2: Private TLS proxy and server proof

**Files:** `infra/private-back-room/compose.yml`, startup/certificate scripts, server proof helper.
**Interfaces:** Private client hostname `briven-customer-db`, port 5432; separate CA/public certificate mounts and independent proxy secret in Dokploy.

- [x] Deploy through Dokploy; check no published ports and required TLS.
- [x] Create two synthetic projects; verify each scoped role, wrong password, wrong tenant, administrator rejection and expiry/revocation.
- [x] Write rows/vectors over the customer connection; restart compute and engine and verify persisted data.
- [x] Dump and restore synthetic customer data into an isolated test database, verify vector search and record evidence.

## Task 3: Website launch prerequisites

- [ ] Replace API/runtime/realtime shared-schema routing with project-scoped engine credentials.
- [ ] Prepare fresh dashboard/Auth schemas and owner account, gate unfinished Auth enrollment, and prove staged login/project creation.
- [ ] Cut over through Dokploy only after all preceding proof passes; retain the previous service/storage and verify the real dashboard.

## Verified on 6 October 2026

Image `sprint3-engine-20261005-3` is running privately on KVM2. The reproducible host proof is `infra/private-back-room/prove-customer.py`, with `customer`, `recovery`, and `persisted` phases. All three phases passed, including complete engine recreation through Dokploy with retained volumes, resumed customer TLS login, persisted rows/vector search, three safekeepers, and fsync enabled. Synthetic customer and catalog backups restored independently and remain root-only under `/var/backups/briven-engine-20261006/`. The original website is still live; no cutover or legacy data import occurred.
