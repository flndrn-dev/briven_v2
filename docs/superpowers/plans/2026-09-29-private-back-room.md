# Private back room — install plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put one empty customer notebook per project in a private back room on the same computer that already serves briven.tech, prove the private test, and stop before the public site changes.

**Architecture:** The website stays the shop window people already use. The engine runs beside it on `187.77.183.190`, on a Docker network that has no route to the internet and no published ports. `briven_local` starts the pageserver, three safekeepers, the storage broker, and the storage controller. `briven_control_api` is the private door. The customer's 15-minute key is not handed out until a TLS proxy is listening on that same closed network. The live website compose is not edited, and its main branch is not pushed, in this plan.

**Tech Stack:** Engine image `ghcr.io/flndrn-dev/briven-engine`, `briven_local`, `briven_control_api`, Bitnami MinIO images named in `.deploy.md`, a private PostgreSQL catalog, Docker Compose with `internal: true`. No Kubernetes. No second machine.

## Startup correction (2026-09-30)

The checked-in files now use `local-env.toml` with three safekeepers and disk sync enabled. `briven_local init` without a config creates only one safekeeper. The startup script uses `--force empty-dir-ok` because it creates the mounted data directory first. `briven_local start` already starts the broker, controller, pageserver, and safekeepers, so separate broker/controller starts in the original Task 2 snippet would make `set -e` exit on an already-running process. Follow the checked-in files and `check-compose.sh` for Tasks 1–2; the older snippets below are a record of the initial draft.

The private image and object-storage configuration still need a live proof. The generic Dockerfile now builds and copies the two Briven control binaries, but no image or private room has been deployed.

## Global Constraints

- Customer databases live on the Briven engine, on the same computer that already serves briven.tech (`187.77.183.190`), in a private back room. The website stays the public shop window.
- Notes have called that computer both KVM2 and KVM4. The choice is that one computer, the one the public site already uses. It is not a second machine, and it is not an outside database company. Before any install, read the server name in the Dokploy panel and use the entry that already runs the website. Do not guess between the two names.
- The website asks the engine only with a signed hall pass that lasts 60 seconds. The approved recipe puts the team, the role, and the person in that pass (`org_id`, `role`, `sub`). The project name stays in the create body `{name}`. Do not put the project name in the pass.
- The customer's own key is random, lasts 15 minutes, opens only that project's database, and travels through TLS. The engine's master login is refused on that door.
- One empty database per project. A finished name for that team is refused. An unfinished or failed create continues the same database. Two teams may use the same project name. One team may not have two databases with that name.
- The project becomes ready only after pgvector is on and a small vector query works. Do not change that existing check. The check remains `CREATE EXTENSION IF NOT EXISTS vector; SELECT '[1,2,3]'::vector(3);` and it uses the engine's own admin login inside the back room. Do not return that login to a customer.
- Do not import old accounts, projects, or customer rows. Old production volumes stay intact. Do not mount the live website volumes or `/var/lib/doltgres`.
- Saved-copy and undo buttons stay off. Do not turn them back on. Do not build branch restore in this plan.
- Real passwords stay in the Dokploy panel. Do not write passwords into git, a log, or a chat reply. Do not publish `docker-compose/docker-compose.yml`.
- The public website stays on its current setup until the private test has passed and a separate go-ahead is given. Do not set `BRIVEN_ENGINE_CONTROL_URL` on the live website compose in this plan.
- Do not push `neondatabase/neon`. Do not push either repo's `main` from this plan. Do not recreate Dokploy compose `LykAyuz6qInYZe37wIMLp`. Do not touch compose `briven-france` (`Qp9Zg2JRqbJ6onE1_n6OO`).
- Saving this file is not permission to install. Task 3 and every task after it wait for a later yes in the conversation.

---

## This plan stops before the public site

Tasks 1 and 2 stay on this Mac. They only add a closed-room file and a check that the file has no open doors. They do not SSH and they do not start Docker on the server.

Tasks 3 through 7 run only after flndrn says yes to install the back room. They still do not change `https://briven.tech`.

Task 8 is a stop. Pushing the website's `main` branch, syncing the local folders onto `main`, and deploying are written there so they are not forgotten. They run only after the private test passes and flndrn gives a separate yes. That yes is not included in "the plan looks right."

## File structure

Engine repo, branch `sprint3-serverless-postgres`, checkout `/Users/flndrn/Desktop/brivendatabase-briven`:

- Create `infra/private-back-room/compose.yml` — the closed room. No host ports. Secrets come from the panel.
- Create `infra/private-back-room/start-private.sh` — starts `briven_local` and the control API inside that room.
- Create `infra/private-back-room/check-compose.sh` — fails if the room file publishes a port, uses the deleted MinIO images, or contains the practice password.
- Do not modify `docker-compose/docker-compose.yml`.
- Do not stage `.deploy.md`, `.neon_clippy_args`, `NOTICE`, `docs/rfcs/YYYY-MM-DD-copy-me.md`, `docs/superpowers/specs/2026-09-27-briven-website-engine-deploy-design.md`, or `.briven_clippy_args`.

Website repo: do not edit it in this plan. Do not check out its `main` branch to do this work.

---

### Task 1: The closed-room check

**Files:**
- Create: `infra/private-back-room/check-compose.sh`
- Test: `infra/private-back-room/check-compose.sh`

**Interfaces:**
- Consumes: nothing.
- Produces: a script that exits 0 only when `infra/private-back-room/compose.yml` and `infra/private-back-room/start-private.sh` are both present and closed. Later tasks must keep that script passing.

- [ ] **Step 1: Write the check**

```bash
#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
file=infra/private-back-room/compose.yml
start=infra/private-back-room/start-private.sh
test -f "$file"
test -f "$start"
if grep -nE '^[[:space:]]*ports:' "$file"; then
  echo "refusing published ports" >&2
  exit 1
fi
if grep -nE 'minio/minio|minio/mc' "$file"; then
  echo "refusing deleted MinIO images" >&2
  exit 1
fi
if grep -nE 'MINIO_ROOT_PASSWORD=password|PGPASSWORD=cloud_admin' "$file" "$start"; then
  echo "refusing a practice password" >&2
  exit 1
fi
if grep -nE 'traefik|Host\(`briven\.tech`\)' "$file"; then
  echo "refusing a public route" >&2
  exit 1
fi
if grep -nE '/var/lib/doltgres' "$file" "$start"; then
  echo "refusing the live database volume" >&2
  exit 1
fi
grep -q 'bitnamilegacy/minio:latest' "$file"
grep -q 'bitnamilegacy/minio-client:latest' "$file"
grep -q 'internal: true' "$file"
grep -q 'briven_local init' "$start"
grep -q 'briven_local start' "$start"
grep -q 'briven_local storage-controller start' "$start"
grep -q 'briven_control_api' "$start"
echo "private room file is closed"
```

- [ ] **Step 2: Run it and see it fail**

Run: `sh infra/private-back-room/check-compose.sh`

Expected: the script exits non-zero because `compose.yml` does not exist yet.

- [ ] **Step 3: Do not commit yet**

Task 2 adds the files this check requires. Commit once, after Task 2 makes the check pass.

---

### Task 2: The closed-room file

**Files:**
- Create: `infra/private-back-room/compose.yml`
- Create: `infra/private-back-room/start-private.sh`
- Test: `infra/private-back-room/check-compose.sh`

**Interfaces:**
- Consumes: the check from Task 1.
- Produces: a compose project named `briven-back-room` on network `briven-back-room` with `internal: true`. Services `minio`, `minio-init`, `catalog`, and `engine`. The engine data directory is `/var/lib/briven` on volume `briven-back-room-data`. No service has a `ports:` key.

- [ ] **Step 1: Write the start script**

`infra/private-back-room/start-private.sh`:

```bash
#!/bin/sh
set -eu
if [ -z "${BRIVEN_CONTROL_IDENTITY_SECRET:-}" ]; then
  echo "missing hall-pass secret" >&2
  exit 1
fi
if [ -z "${BRIVEN_CONTROL_DATABASE_URL:-}" ]; then
  echo "missing catalog database" >&2
  exit 1
fi
export BRIVEN_REPO_DIR=/var/lib/briven
export BRIVEN_ENV=production
export BRIVEN_CONTROL_API_BIND=0.0.0.0:8787
export BRIVEN_LOCAL_BIN="${BRIVEN_LOCAL_BIN:-briven_local}"
mkdir -p "$BRIVEN_REPO_DIR"
if [ -z "$(ls -A "$BRIVEN_REPO_DIR" 2>/dev/null || true)" ]; then
  briven_local init
fi
briven_local start
briven_local storage-controller start
briven_local storage-broker start
exec briven_control_api
```

`chmod 755 infra/private-back-room/start-private.sh`

The bind address is inside the closed network. It is safe only because Task 1 forbids a `ports:` key. Do not change the bind to the host's public address.

- [ ] **Step 2: Write the compose file**

`infra/private-back-room/compose.yml`:

```yaml
# Private back room. No host ports. Secrets come from the Dokploy panel.
# Do not deploy docker-compose/docker-compose.yml.
name: briven-back-room

services:
  minio:
    image: bitnamilegacy/minio:latest
    user: "0:0"
    entrypoint: ["/opt/bitnami/minio/bin/minio"]
    command: ["server", "/data", "--address", ":9000", "--console-address", ":9001"]
    environment:
      MINIO_ROOT_USER: ${MINIO_ROOT_USER:?set in the Dokploy panel}
      MINIO_ROOT_PASSWORD: ${MINIO_ROOT_PASSWORD:?set in the Dokploy panel}
    volumes:
      - briven-back-room-minio:/data
    networks:
      - briven-back-room
    restart: unless-stopped

  minio-init:
    image: bitnamilegacy/minio-client:latest
    depends_on:
      - minio
    environment:
      MINIO_ROOT_USER: ${MINIO_ROOT_USER:?set in the Dokploy panel}
      MINIO_ROOT_PASSWORD: ${MINIO_ROOT_PASSWORD:?set in the Dokploy panel}
    entrypoint: ["/bin/sh", "-c"]
    command:
      - |
        set -eu
        mc=/opt/bitnami/minio-client/bin/mc
        until "$$mc" alias set room http://minio:9000 "$$MINIO_ROOT_USER" "$$MINIO_ROOT_PASSWORD"; do
          sleep 1
        done
        "$$mc" mb --ignore-existing room/briven
    networks:
      - briven-back-room
    restart: "no"

  catalog:
    image: postgres:16
    environment:
      POSTGRES_USER: briven_catalog
      POSTGRES_PASSWORD: ${BRIVEN_CONTROL_DB_PASSWORD:?set in the Dokploy panel}
      POSTGRES_DB: briven_control
    volumes:
      - briven-back-room-catalog:/var/lib/postgresql/data
    networks:
      - briven-back-room
    restart: unless-stopped

  engine:
    image: ghcr.io/flndrn-dev/briven-engine:${BRIVEN_ENGINE_TAG:-latest}
    depends_on:
      - minio-init
      - catalog
    environment:
      BRIVEN_ENV: production
      BRIVEN_REPO_DIR: /var/lib/briven
      BRIVEN_CONTROL_API_BIND: 0.0.0.0:8787
      BRIVEN_CONTROL_IDENTITY_SECRET: ${BRIVEN_CONTROL_IDENTITY_SECRET:?set in the Dokploy panel}
      BRIVEN_CONTROL_DATABASE_URL: postgres://briven_catalog:${BRIVEN_CONTROL_DB_PASSWORD:?set in the Dokploy panel}@catalog:5432/briven_control
      MINIO_ROOT_USER: ${MINIO_ROOT_USER:?set in the Dokploy panel}
      MINIO_ROOT_PASSWORD: ${MINIO_ROOT_PASSWORD:?set in the Dokploy panel}
    volumes:
      - briven-back-room-data:/var/lib/briven
      - ./start-private.sh:/start-private.sh:ro
    command: ["/bin/sh", "/start-private.sh"]
    networks:
      - briven-back-room
    restart: unless-stopped

networks:
  briven-back-room:
    internal: true

volumes:
  briven-back-room-minio:
  briven-back-room-catalog:
  briven-back-room-data:
```

- [ ] **Step 3: Run the check**

Run: `sh infra/private-back-room/check-compose.sh`

Expected: `private room file is closed` and exit 0.

- [ ] **Step 4: Commit only these files**

```bash
git add infra/private-back-room/check-compose.sh \
  infra/private-back-room/compose.yml \
  infra/private-back-room/start-private.sh \
  docs/superpowers/plans/2026-09-29-private-back-room.md
git commit -m "docs: Record the private back room steps"
```

Stay on `sprint3-serverless-postgres`. Do not push.

---

### Task 3: Look at the real computer, then stop if the name is unclear

**Files:**
- Modify: none
- Test: the panel read below

**Interfaces:**
- Consumes: `.deploy.md` facts. Website compose id `LykAyuz6qInYZe37wIMLp`. Live file `infra/dokploy/compose.dokploy.yml`. Server address `187.77.183.190`.
- Produces: one written line in the task report naming the Dokploy server entry that already runs that compose. Later tasks use that entry and no other.

This task starts only after flndrn says yes to install.

- [ ] **Step 1: Read the panel**

Sign in to `https://admin.loowii.com`. Open the Briven project, compose `briven-website`, id `LykAyuz6qInYZe37wIMLp`. Record the server entry that compose already uses.

Expected: one server, the machine `187.77.183.190`. `.deploy.md` says the website service was created on `kvm2-server` and also calls the Dokploy name `KVM4`. Trust the panel entry attached to compose `LykAyuz6qInYZe37wIMLp`. If the panel shows a different machine, stop. Do not add a server. Do not recreate the compose.

- [ ] **Step 2: Confirm no website deploy is running**

If that compose status is `running`, stop. Do not start a second build. Do not install the back room while a website build is running.

- [ ] **Step 3: Pull the engine image**

On that server, with the key named in `.deploy.md`:

```bash
docker pull ghcr.io/flndrn-dev/briven-engine:latest
```

Expected: the pull finishes. If it fails, stop. Do not build the image on the server. Do not substitute `docker-compose/docker-compose.yml`.

---

### Task 4: Start the closed room

**Files:**
- Modify: none on the live website
- Test: the checks in this task

**Interfaces:**
- Consumes: Task 2 files and the server entry from Task 3.
- Produces: containers `minio`, `catalog`, and `engine` on network `briven-back-room`, restarting unless stopped. The control API answers only inside that network.

- [ ] **Step 1: Put the secrets in the panel**

In the Dokploy environment for this new compose only, set `MINIO_ROOT_USER`, `MINIO_ROOT_PASSWORD`, `BRIVEN_CONTROL_DB_PASSWORD`, and `BRIVEN_CONTROL_IDENTITY_SECRET`. Generate them in the panel. Do not copy them into git or the chat. Do not reuse the practice value. The identity secret must be the same value the website will use later, and it stays in the panel until the separate public yes.

- [ ] **Step 2: Start the room**

From a copy of the engine branch on the server, not from the practice compose:

```bash
docker compose -f infra/private-back-room/compose.yml up -d
```

Expected: no error. `docker compose -f infra/private-back-room/compose.yml ps` shows `minio`, `catalog`, and `engine` up. `minio-init` has exited 0.

- [ ] **Step 3: Prove there is no public door**

```bash
docker compose -f infra/private-back-room/compose.yml port engine 8787
docker compose -f infra/private-back-room/compose.yml port minio 9000
docker compose -f infra/private-back-room/compose.yml port catalog 5432
```

Expected: each command reports that the port is not published. From a laptop, `curl` to `http://187.77.183.190:8787` does not reach the control API.

- [ ] **Step 4: Prove the private door answers**

```bash
docker compose -f infra/private-back-room/compose.yml exec engine \
  wget -qO- http://127.0.0.1:8787/health || true
```

If that binary has no `wget`, use the image's existing HTTP client. Expected: a response from the control API, not a connection error. If `briven_local` is missing from the image, stop and report that. Do not open a port to debug it.

Inside the engine container, `ps` includes `pageserver`, three `safekeeper` processes, `storage_broker`, `storage_controller`, and `briven_control_api`. If `storage-broker` is already started by `briven_local start`, a second start that says it is already running is acceptable. A missing pageserver or fewer than three safekeepers is a failure. Stop and leave the room up only if the processes are the ones named here. Do not switch the website.

---

### Task 5: Catalog backup before any customer notebook

**Files:**
- Modify: none
- Test: the dump and the failed-migration check below

**Interfaces:**
- Consumes: the `catalog` service from Task 4.
- Produces: a dump file on the server under `/var/backups/briven-back-room/`, outside the website volumes. A failed migration on a copy of that catalog leaves the copy at the previous version.

- [ ] **Step 1: Dump the empty catalog**

```bash
mkdir -p /var/backups/briven-back-room
docker compose -f infra/private-back-room/compose.yml exec -T catalog \
  pg_dump -U briven_catalog -d briven_control \
  > /var/backups/briven-back-room/catalog-before-test.sql
```

Expected: the file exists and is not empty. Do not commit it.

- [ ] **Step 2: Prove a bad change rolls back**

Run this against a new database `briven_control_copy` created from the dump, not against `briven_control` and not against the website database:

```bash
docker compose -f infra/private-back-room/compose.yml exec -T catalog \
  psql -U briven_catalog -d postgres -c 'CREATE DATABASE briven_control_copy'
docker compose -f infra/private-back-room/compose.yml exec -T catalog \
  psql -U briven_catalog -d briven_control_copy \
  < /var/backups/briven-back-room/catalog-before-test.sql
docker compose -f infra/private-back-room/compose.yml exec -T catalog \
  psql -U briven_catalog -d briven_control_copy -v ON_ERROR_STOP=1 -c \
  'BEGIN; CREATE TABLE rollback_probe(id int); SELECT 1/0; COMMIT;'
```

Expected: the third command fails. Then:

```bash
docker compose -f infra/private-back-room/compose.yml exec -T catalog \
  psql -U briven_catalog -d briven_control_copy -c '\dt rollback_probe'
```

Expected: the probe table is absent. Drop `briven_control_copy` after that. Do not drop `briven_control`.

---

### Task 6: The locked customer door

**Files:**
- Modify: `infra/private-back-room/compose.yml` only if the proxy binary's own help shows a listen flag that can bind inside the network
- Test: master login is refused, and a customer URI is not returned while the proxy is absent

**Interfaces:**
- Consumes: `BRIVEN_CUSTOMER_PROXY_HOST` and `BRIVEN_CUSTOMER_PROXY_PORT` read by `briven_control_api`. While the host is unset, the door returns HTTP 503 and the words `customer database credentials are not available yet`.
- Produces: either a proxy listening only on network `briven-back-room`, or a stopped task. It does not produce a public port.

- [ ] **Step 1: Confirm the door stays shut**

With `BRIVEN_CUSTOMER_PROXY_HOST` unset, ask the control API for a connection using a hall pass minted by `signHallPass` in `apps/api/src/services/engine-project.ts`. Do not print the pass.

Expected: HTTP 503 and no `cloud_admin` string in the body.

- [ ] **Step 2: Find the proxy binary**

Inside the engine image:

```bash
proxy --help
```

The package path is `proxy/src/bin/proxy.rs`. If the binary is absent, stop this task and say the locked door cannot be installed from this image. Leave `BRIVEN_CUSTOMER_PROXY_HOST` unset. Do not publish port 5432.

- [ ] **Step 3: Start the proxy only inside the room**

Add a `proxy` service to `infra/private-back-room/compose.yml` using the listen flag from that help text. Bind it to the private network. Give it no `ports:` key. Set `BRIVEN_CUSTOMER_PROXY_HOST=proxy` and `BRIVEN_CUSTOMER_PROXY_PORT` to the port that help text listens on, on the engine service only.

Run `sh infra/private-back-room/check-compose.sh` again. Expected: `private room file is closed`.

Then recreate the engine and proxy containers. From another container on `briven-back-room`, a TCP connection to `proxy` on that port succeeds. From a laptop, that port on `187.77.183.190` does not accept a connection.

---

### Task 7: The private test

**Files:**
- Modify: none
- Test: the seven checks from `docs/superpowers/specs/2026-09-29-customer-database-home-design.md`

**Interfaces:**
- Consumes: the private control API at `http://engine:8787` on network `briven-back-room`, and the proxy from Task 6.
- Produces: one empty test notebook, then the room left running. No website row is required. Do not point `briven.tech` at this door.

Run every check from a container on `briven-back-room`. Mint each hall pass with `signHallPass` for 60 seconds. The body is JSON `{"name":"private-test"}`. Do not print secrets.

- [ ] **Step 1: One empty notebook**

POST `/v1/projects`. Expected: the project becomes ready only after the vector query works, and one tenant exists. A second POST with the same name returns HTTP 409.

- [ ] **Step 2: An unfinished create stays one notebook**

Stop the engine container after create has started and before it is ready. Start it again. POST the same name. Expected: HTTP 200 and the same tenant, not a second database.

- [ ] **Step 3: Look-only and a removed pass**

A hall pass with role `viewer` cannot create and cannot get a write key. Expected: HTTP 403 `insufficient organization role`. A pass that is already expired gets HTTP 401.

- [ ] **Step 4: A key opens only its own notebook**

Ask for the connection on the ready project. Expected: the URI starts with `postgresql://briven_`, contains `sslmode=require`, and does not contain `cloud_admin`. Connect with that key and insert one row. Use the same password against a second project. Expected: the second connection fails.

Try the login `cloud_admin` on the proxy. Expected: refusal.

- [ ] **Step 5: Restart keeps the row**

Restart the engine service and the project compute. Connect again with a new 15-minute key for the same project. Expected: the row from Step 4 is still there.

- [ ] **Step 6: Write the result**

Record pass or fail for each step in the task report. Any failure stops the plan. Do not continue to Task 8's ship steps. The public site stays on the current setup.

---

### Task 8: Stop. The shop door is a separate yes

**Files:**
- Modify: none until a later conversation says yes

**Interfaces:**
- Consumes: a passed Task 7.
- Produces: nothing on the public site.

- [ ] **Step 1: Stop and ask**

Tell flndrn, in plain words, that the private test passed or which step failed. Do not push. Do not set `BRIVEN_ENGINE_CONTROL_URL` on compose `LykAyuz6qInYZe37wIMLp`. Do not replace `infra/dokploy/compose.dokploy.yml` with `infra/dokploy/compose.serverless-postgres.yml`.

- [ ] **Step 2: Only after a new yes, ship**

This step is not authorized by approving this plan. After that separate yes, and only then:

1. Commit any uncommitted back-room files on `sprint3-serverless-postgres` in each repo. Do not commit the unrelated dirty engine files listed above. Do not commit passwords.
2. Push the engine with `git push origin HEAD:main` from the engine repo. Never push `upstream` (`neondatabase/neon`). This saves the engine code. It does not publish a public database.
3. Push the website with `git push origin HEAD:main` from the website repo. That push rebuilds the live site by itself. If a deploy is already `running`, do not start another. If the push does not start one, call Dokploy `compose.deploy` once for compose id `LykAyuz6qInYZe37wIMLp`, as `.deploy.md` says.
4. Sync the local folders: in the engine checkout and in the lasting website checkout (not only `/tmp`), check out `main` and fast-forward to the pushed commits.
5. Prove the shop: `curl -sI https://briven.tech` returns a page, and `curl -sS https://api.briven.tech/ready` succeeds. A panel status of `done` is not the proof.
6. Set `BRIVEN_ENGINE_CONTROL_URL` to the private engine address only as part of this same yes, and only if Task 7 passed. The matching identity secret stays in the panel.

If Task 7 has not passed, skip this step entirely.
