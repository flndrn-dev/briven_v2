# PostgreSQL, branches and vectors: reviewed durability acceptance

Completed on 14 October 2026. The selected launch gap moves from **95% to 100%**
against the agreed failure, restart and durability checklist. The owner explicitly
selected reviewed restart and kept automatic recovery as a separate gap.

## Fix and accepted release

The drill reproduced two PID-namespace restart failures: the controller mistook
a safekeeper's recycled PID for its PostgreSQL process, and a compute refused to
start because its old Unix socket lock survived SIGKILL. The startup guard now
verifies process, data-directory, PID and socket identity before clearing proven
stale runtime locks. It preserves live PostgreSQL processes, durable files and
compute crash markers. Unknown identities and foreign locks are refused.

The fix and harness are committed on main at
`628bcab5b52ffadc0b281f7e69e2b60bf49287a4`. Dokploy's existing
Briven / production / engine service on `briven.tech` reached **Done** at that
commit. The one-time engine-only recreate command was restored to the service's
normal command afterward; resource and public PostgreSQL overlays are retained.

| Accepted component | Exact identity |
| --- | --- |
| Compiled engine source | `743cab2d44712ad3d81f75563c98bf6c06f5f427` |
| Engine manifest | `sha256:4bf8842f589e7c774d1742c80f9789964152ac7b86295c08ee7f804305c4e9a8` |
| Actual engine image configuration | `sha256:b045b0c105ce8598f83205d094d7803bf7b624fb66b3bb2a751e62fae2f5eaac` |
| Mounted startup script SHA256 | `b4ac62d0fc57c8073cc99a6e4f7f91b6764caa3740e1b501a8767f13219ab4a5` |

The script is mounted from the Dokploy checkout, so these startup fixes require
no replacement of the already-tested compiled engine image.

## Evidence

[Successful CI run 38582956268](https://github.com/flndrn-dev/briven_v2/actions/runs/38582956268)
used the exact immutable engine artifact and accepted startup script in fresh
bounded containers on a disposable Linux runner. It published only redacted
evidence and summary files. Fourteen restart-guard tests and ten fault/diagnostic
guard tests pass. The real matrix contains 751 evidence records and 51 check types;
both `passed` and `cleanupPassed` are true.

| Acceptance | Result |
| --- | --- |
| Three synchronous safekeepers; acknowledged WAL actually flushed on a majority | Pass |
| Independent original/child writes; stable project, timeline and compute identities; same retry reused and conflicting retry refused | Pass |
| Graceful full-engine restart; exact row/vector checksums and valid HNSW cosine indexes retained | Pass |
| Two SIGKILLs with reviewed recovery; acknowledged writes retained and uncommitted transaction absent | Pass |
| Eight simultaneous hostname-verified TLS clients per branch; identical data and expected indexed nearest neighbor | Pass |
| One safekeeper unavailable; writes acknowledged by the remaining quorum | Pass |
| Two safekeepers unavailable; new COMMIT unacknowledged for five seconds, then the same COMMIT finished exactly once after restoration | Pass |
| Object store paused during writes; restored store plus second crash preserved every acknowledged record | Pass |
| Final reads/writes; viewer and foreign-organization refusal; no duplicated resources | Pass |
| SQL clients closed; owned services stopped; unrelated resources preserved | Pass |

Final original branch: **1,030 rows**, SHA256
`ca656f68ecb23748315fe1ab250afd3afb75822fb590c97325c12784830f6198`.
Final child branch: **1,029 rows**, SHA256
`fbe7dbec14d3e16672a389f9acd254666a1e7ebd7bfcdcf555090c6312e3e7d1`.
Checksums cover canonical IDs, payload hashes and vector values; every new write
also verified preservation of the previously acknowledged contents.

Artifact: `postgres-durability-38582956268-1`, archive digest
`sha256:cc207f27dd7702b7ab32d4b029df679a2e283925f7aeaace37f79d224e16bd7c`.
The downloaded evidence and all failed runs are retained locally under
`.local/postgres-durability-20261014/`. Failed runs were not counted as acceptance.

## Live verification and scope

Read-only before/after checks verified the deployed script hash, healthy engine,
unchanged image, three project records, four compute definitions, environment
hash, mounts, ports and all 33 other running container identities. Both protected
Mittera projects retain their original bindings and deletion states. Website and
engine health checks passed, public HTTPS readiness returned 200, and private
operator routes remained private.

An existing synthetic customer's normal lease connected externally to
`briven.tech` with system-trusted TLS and hostname verification, the exact customer
role, and its retained `[1,2,3]` vector row. Its temporary session was revoked.
No production fault was injected and no customer data or volume was deleted.

Reviewed recovery uses the existing native operator command only after verifying
that the two fixture computes are absent and their retained definitions and
distinct timelines match. The legacy CLI's reviewed `--allow-multiple` override
permits restarting its own crashed endpoint; the fixture's strict checks exclude
any second primary. No SQL write is retried after an ambiguous outcome, and no
failure record or lifecycle policy is cleared.

Durability is measured at the synchronous, quorum-flushed safekeeper WAL boundary.
Compute-local disks are disposable. Automatic recovery, backup retention,
physical disk/host destruction and cross-region failover remain separate tasks.
