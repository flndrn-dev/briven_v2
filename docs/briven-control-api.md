# Briven Control API (Local Development)

The Briven control API turns local engine operations into project, branch, compute,
and connection workflows. It is a development adapter for one local Briven engine
environment. It is not a hosted customer API yet: it has one operator token, one
machine's project registry, and loopback connection strings.

## Start

Build both binaries:

```sh
cargo build -p control_plane --bin briven_local --bin briven_control_api
```

Initialize the engine using `briven_local init`. With a debug build, start
services using `RUST_LOG=info ./target/debug/briven_local start`; more
restrictive filters can suppress tracing spans the debug engine checks.
Then set a random operator token (at least 32 characters) and start the API:

```sh
export BRIVEN_CONTROL_API_TOKEN="$(openssl rand -hex 32)"
./target/debug/briven_control_api
```

The API listens on `127.0.0.1:8787` by default. Set `BRIVEN_CONTROL_API_BIND`
to another loopback address if needed. `BRIVEN_REPO_DIR` selects the initialized
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

The API records `provisioning`, `ready`, or `failed` for each project in
`product-projects.json` inside the local engine directory. It reports an engine
failure as an error and keeps the failed record for diagnosis. Connection URIs
are returned only after a compute answers a real SQL query and carry
`environment: "local"`. The API chooses free local ports for each compute.

The returned local connection uses the engine's `cloud_admin` role. It is a
development credential and must not be exposed to customers.

## Hosted API Work Still Required

- Customer sign-in and organization ownership on every route.
- A durable transactional project catalog and recovery/retry for partial operations.
- Production compute scheduling, credentials, TLS, and public connection routing.
- Provisioning reconciliation across controller and API restarts.
- Limits and billing enforcement before inviting paid customers.

The dashboard should use this API shape, but it must not treat the local adapter
or its loopback URLs as a hosted production service.
