# PostgreSQL, branch and vector durability acceptance

The owner selected this single launch gap for completion before the next task.
Continue the accepted native workflow and isolated-engine design. No production
restart, new public route, customer drill or global policy activation is included.

## Acceptance boundary

Use a disposable GitHub-hosted Linux runner and the exact published production
engine image: source `743cab2d44712ad3d81f75563c98bf6c06f5f427`, manifest
`sha256:4bf8842f589e7c774d1742c80f9789964152ac7b86295c08ee7f804305c4e9a8`.
Use the existing production startup/configuration, including three synchronous
safekeepers and pageserver `no_sync=false`. Keep compute lifecycle disabled.
The reproduced PID-namespace restart defect is fixed in `start-private.sh`:
before any services start, validate the recorded data/port/socket identities,
inspect the namespace for a live bound controller postmaster, and clear only
proven stale controller PID/socket locks and owned compute socket locks. Keep
compute PID files as unexpected-exit markers, preserving reviewed recovery.
Preserve durable files and fail closed on unknown
process or foreign lock identity. Record the actual startup-script SHA256 in
CI evidence and verify that same script is installed through Dokploy before
closing acceptance; the immutable engine binary remains743.
The fixture receives fresh private secrets, its own catalog, object store,
repository and TLS volumes, one internal network and no published ports.
Engine cap:4GiB,1.5CPUs,512tasks; catalog256MiB/.25CPU/128tasks;
object store512MiB/.25CPU/128tasks. No production host-control socket is mounted.

The client validates the fixture CA and proxy hostname. It uses actual signed
owner-scoped engine routes and customer PostgreSQL leases. Create one generated
project and a writable child branch. Record exact timeline/endpoint identities,
acknowledged row/vector checksums and a valid HNSW cosine index before faults.

Pass all of the following on the matching immutable image:

1. The durable WAL path is enabled (three sync=true safekeepers, durable catalog, synchronous_commit=on and walproposer); original and child branches accept independent
   committed writes. Current viewer/foreign-organization requests are refused.
2. Graceful full-engine stop/start preserves identities, exact committed
   contents and valid vector indexes, and new writes still work.
3. SIGKILL of only the owned fixture engine preserves acknowledged commits and
   removes an explicitly uncommitted transaction. Recovery produces no duplicate
   project, timeline or endpoint. Eight concurrent TLS connections read the same
   recovered contents and the expected nearest-neighbor result. Abrupt compute
   exits deliberately require operator review in the current product. After the
   full engine is healthy, verify no compute process remains alive, revalidate
   the two saved endpoint/tenant/timeline definitions, and use the existing
   `briven_local endpoint start` operator command. This reconstructs disposable
   compute caches from durable storage. The legacy CLI counts its own crashed
   endpoint as a conflicting primary, so the reviewed command uses its existing
   `--allow-multiple` override only after proving there are exactly two retained
   definitions, two distinct timelines and no live compute processes. No extra
   primary is created. Do not clear failure records, retry
   writes, change lifecycle policy or claim automatic crash recovery.
4. One unavailable safekeeper still permits acknowledged writes with the other
   two. With two safekeepers paused, a new COMMIT must not acknowledge during a
   five-second observation. Restoring quorum permits that same COMMIT to finish
   exactly once; no retry of an ambiguous write is allowed.
5. A brief fixture object-store outage does not corrupt acknowledged data.
   After restoring it and another forced engine crash, both branch checksums and
   indexed vector queries remain correct.
6. Final reads/writes and permission checks pass; all fixture connections close
   and its containers stop. Only redacted evidence is archived. Production
   image/container identities, customer bindings and health remain unchanged.

This closes the specified launch acceptance checklist. Host destruction,
physical disk loss, cross-region failover and backup retention belong to their
separate infrastructure/saved-copy requirements and are not proved by a process
crash with retained volumes.

## Failure handling

The harness runs only on a GitHub-hosted Linux runner. Every fault validates the
exact run namespace, purpose label, image and owned mounts before targeting a
container or PID. It rejects a production engine name or any host bind outside
the checked-out fixture source. Marker exchanges have finite timeouts; cleanup
restores paused components and stops only verified fixture containers.
Lease/password/JWT/key material never enters evidence or command arguments.
Failed acceptance remains failed. Any product defect gets a meaningful
regression and scoped fix before re-running this matrix on its matching image.

Compute-local fsync is intentionally off in the inherited disaggregated engine; quorum-flushed safekeeper WAL is its durability boundary, as documented in `pgxn/neon/walproposer_pg.c`. Do not change or miscertify this as ordinary local-disk PostgreSQL durability.

References: [WAL quorum protocol](https://github.com/neondatabase/neon/blob/main/docs/safekeeper-protocol.md), [PostgreSQL WAL settings](https://www.postgresql.org/docs/16/runtime-config-wal.html),
[pgvector index documentation](https://github.com/pgvector/pgvector).
