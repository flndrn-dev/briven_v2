# Briven Control API

The Briven control API turns local engine operations into project, branch, compute,
and connection workflows. It still controls one local Briven engine environment;
hosted compute scheduling and customer-safe database credentials are pending.
Signed customer mode enables pgvector on each created compute and verifies the
vector type before reporting project creation as ready.

## Start

Build both binaries:

```sh
cargo build -p control_plane --bin briven_local --bin briven_control_api
```

Initialize the engine using `briven_local init`. With a debug build, start
services using `RUST_LOG=info ./target/debug/briven_local start`; more
restrictive filters can suppress tracing spans the debug engine checks.
For a single local operator, set a random token (at least 32 characters) and
start the API:

```sh
export BRIVEN_CONTROL_API_TOKEN="$(openssl rand -hex 32)"
export BRIVEN_ENV=development
./target/debug/briven_control_api
```

That legacy token belongs to the `local` organization. To use separate
organization keys instead, unset `BRIVEN_CONTROL_API_TOKEN` and set
`BRIVEN_CONTROL_API_KEYS_JSON` to a JSON object mapping organization identifiers
to distinct random keys of at least 32 characters. Only one credential variable
may be set. These keys are development credentials, not customer login sessions.
Static keys are rejected when `BRIVEN_ENV` is not `development`.

For customer access, set the same random secret of at least 32 characters as
`BRIVEN_CONTROL_IDENTITY_SECRET` on the website API and this engine process.
Set `BRIVEN_CONTROL_DATABASE_URL` to PostgreSQL; signed customer mode refuses
the local JSON catalog. Do not also set either static key variable. On the website API, set
`BRIVEN_ENGINE_CONTROL_URL` to the engine's private service URL and apply
`0057_org_control_keys.sql` to its control database. The API accepts a customer
session or a `bck_` organization key at `/v1/control/projects...`, checks active
organization membership for sessions and active key status for keys, then sends
a 60-second signed assertion to the engine. Only owner/admin members can manage
keys through `/v1/orgs/{id}/control-keys`; key plaintext is returned only on
creation or rotation. Keep the engine on a private network. A non-loopback bind
is allowed only with signed identity configured. The gateway does not expose
the connection URI endpoint, because it still returns a local `cloud_admin`
credential. In signed customer mode the engine also disables that endpoint.

By default, project metadata remains in `product-projects.json` inside the
local engine directory. Set `BRIVEN_CONTROL_DATABASE_URL` to use a PostgreSQL
catalog instead. Loopback PostgreSQL uses a local connection; remote hosts always
use Rustls with system-trusted certificate and hostname validation, and the API
requires TLS instead of falling back to plaintext. The API applies transactional,
versioned migrations under a PostgreSQL advisory lock at startup. Migration 1
adopts an existing `briven_control.projects` table without deleting rows. The API
refuses to start against a newer catalog version. The catalog does **not** automatically import the JSON
file; migrate existing projects deliberately before switching storage. Both
catalog modes scope project names and lookups to the authenticated organization.

The engine listens on `127.0.0.1:8787` by default. Set `BRIVEN_CONTROL_API_BIND`
to the appropriate private interface if needed. `BRIVEN_REPO_DIR` selects the initialized
local engine directory. `BRIVEN_LOCAL_BIN` can point to a different `briven_local`
binary. Every `/v1` request needs `Authorization: Bearer <token>`.

## Routes

| Method | Path | Result |
| --- | --- | --- |
| `GET` | `/healthz` | API process health |
| `GET` | `/v1/projects` | Project names, IDs, provisioning states |
| `POST` | `/v1/projects` | Create a real tenant, main timeline, and running compute |
| `GET` | `/v1/projects/{id}` | One project |
| `GET` | `/v1/projects/{id}/branches` | Branches from engine mappings |
| `POST` | `/v1/projects/{id}/branches` | Create a real copy-on-write timeline |
| `GET` | `/v1/projects/{id}/computes` | Compute endpoints and observed status |
| `POST` | `/v1/projects/{id}/branches/{branch}/computes` | Start a compute for a branch |
| `GET` | `/v1/projects/{id}/branches/{branch}/connection` | Connection URI for a running branch compute |

Project creation body: `{"name":"my-project"}`. Branch creation body:
`{"name":"preview","parent":"main"}`; `parent` defaults to `main`.
Names use lowercase letters, digits, and hyphens, start with a letter, end
without a hyphen, and are at most 48 bytes.

The API records `provisioning`, `ready`, or `failed` for each project in the
selected catalog. It reports an engine failure as an error and keeps the failed
record for diagnosis. Sending the same project name again, in the same
organization, does not create a second tenant. A `ready` name returns conflict.
A `provisioning` or `failed` name continues the saved tenant and main timeline,
starts the existing compute when it is not already running, and returns that
same project. Connection URIs are returned only after a compute answers
a real SQL query and carry `environment: "local"`. The API chooses free local
ports for each compute.

The returned local connection uses the engine's `cloud_admin` role. It is a
development credential and must not be exposed to customers.

## Hosted API Work Still Required

- Live customer-to-engine integration proof and deploy configuration.
- Hosted PostgreSQL catalog backup and recovery/retry for partial operations.
  The local JSON fallback is not transactional.
- Production compute scheduling, credentials, TLS, and public connection routing.
- Hosted provisioning reconciliation across controller and API restarts. The
  local API already resumes one saved tenant after an interrupted create.
- Limits and billing enforcement before inviting paid customers.

The dashboard should use this API shape, but it must not treat the local adapter
or its loopback URLs as a hosted production service.
