# Customer database home — local software contract

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A signed-in team member can create one empty customer database through the private engine, and the engine can mint a 15-minute key that opens only that project's database.

**Architecture:** The website stays the shop window. When `BRIVEN_ENGINE_CONTROL_URL` and `BRIVEN_CONTROL_IDENTITY_SECRET` are both set, Create a project sends a 60-second hall pass and stores the engine's tenant id. The engine's customer door refuses look-only people and refuses the engine master login. Until the proxy address is set, that door still answers that the key is not available, and it never returns the master login. Practice on this Mac, with the engine address unset, keeps today's shared-admin path.

**Tech Stack:** Rust control API (`control_plane`, axum, tokio-postgres, rand 0.9), website API (Bun, Hono, Drizzle, jose). Tests are `cargo test` for the engine binary and `bun test` for the website pure module.

## Global Constraints

- Customer databases live on the Briven engine, on the same computer that already serves briven.tech (`187.77.183.190`), in a private back room. The website stays the public shop window.
- Notes have called that computer both KVM2 and KVM4. The choice is that one computer, the one the public site already uses. It is not a second machine, and it is not an outside database company. Before any install, read the server name in the Dokploy panel and use the entry that already runs the website. Do not guess between the two names.
- The website asks the engine only with a signed hall pass that lasts 60 seconds. The pass names the team and the project. The engine's master key never crosses the public website.
- The customer's own key is random, lasts 15 minutes, opens only that project's database, and travels through TLS. The engine's master login is refused on that door.
- One empty database per project. A finished name for that team is refused. An unfinished or failed create continues the same database. Two teams may use the same project name. One team may not have two databases with that name.
- The project becomes ready only after pgvector is on and a small vector query works. Do not change that existing check.
- Do not import old accounts, projects, or customer rows. Old production volumes stay intact.
- Saved-copy and undo buttons stay off. Do not turn them back on.
- Real passwords stay in the Dokploy panel. Do not write passwords into git, and do not publish the practice compose.
- The public website stays on its current setup until the private test in the spec has passed and a separate go-ahead is given.
- Every task below also follows these rules.

---

## This plan stops before the live computer

The spec covers two separate pieces of work. This file is only the first. It produces software that can be tested on this Mac and against a disposable PostgreSQL, with no install on `187.77.183.190`.

The second plan is not written here and must not be started from these tasks. It is the hot-zone work: reading the Dokploy panel for the server entry that already runs the website, then installing the storage controller, pageserver, at least three safekeepers, object storage, scheduler, private API, and TLS Postgres proxy on that computer, then the private test list in `docs/superpowers/specs/2026-09-29-customer-database-home-design.md`, and only after a separate go-ahead any public cutover. Do not SSH, open ports, edit Dokploy, or deploy from this plan.

Do not edit `/Users/flndrn/projects/briven` or `/Users/flndrn/projects/briven-rewrite`. Do not push `neondatabase/neon`. Do not push either repo's `main`. Engine commits go on `sprint3-serverless-postgres` in this checkout. Website commits go on `sprint3-serverless-postgres` in the website checkout. Do not stage the pre-existing dirty engine files (`.deploy.md`, `.neon_clippy_args`, `NOTICE`, `docs/rfcs/YYYY-MM-DD-copy-me.md`, `docs/superpowers/specs/2026-09-27-briven-website-engine-deploy-design.md`, `.briven_clippy_args`). Do not run rustfmt on the repo. Do not run `drizzle-kit generate`.

## File structure

Engine, this repo:

- Create `control_plane/src/briven_control_api/customer_credential.rs` — pure key rules: role name, 15-minute stamp, SQL text, customer URI, and the door decision. No cluster and no environment variables.
- Modify `control_plane/Cargo.toml` — add the workspace `rand` crate.
- Modify `control_plane/src/bin/briven_control_api.rs` — declare the module, decide the customer door, apply the role SQL only after that decision says issue, and never return `cloud_admin` in customer mode.

Website, checkout of `flndrn-dev/briven-website` branch `sprint3-serverless-postgres` (if `/tmp/briven-website-sprint3` is gone: `gh repo clone flndrn-dev/briven-website /tmp/briven-website-sprint3` then `git checkout sprint3-serverless-postgres`). Do not work from website `main`.

- Create `apps/api/src/services/engine-project.ts` — pure decisions for the engine name, the hall pass, the create answer, who may create, and which control route is allowed.
- Create `apps/api/src/services/engine-project.test.ts` — bun tests. No database.
- Modify `apps/api/src/db/schema.ts` — nullable `engineProjectId`.
- Create `apps/api/drizzle/migrations/0058_engine_project_id.sql` and append journal idx 44.
- Modify `apps/api/src/services/projects.ts` — call the engine only when both engine settings are set.
- Modify `apps/api/src/services/orgs.ts` — read the caller's team role.
- Modify `apps/api/src/routes/projects.ts` — look-only people cannot create; a taken finished name is HTTP 409.
- Modify `apps/api/src/routes/engine-control.ts` — allow the connection path and refuse a look-only read of it.

Leave `existing_name`, `enable_vector`, `resolveControlKey`, and `genSlug` behavior in place. `genSlug` stays the practice-path slug. The engine path must not use it as the engine name.

---

### Task 1: Customer key rules

**Files:**
- Create: `control_plane/src/briven_control_api/customer_credential.rs`
- Modify: `control_plane/Cargo.toml` (add `rand.workspace = true` after `pem.workspace = true`)
- Modify: `control_plane/src/bin/briven_control_api.rs` (module declaration only in this task)
- Test: inside `customer_credential.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces:
  - `pub const CUSTOMER_KEY_SECONDS: u64 = 900`
  - `pub struct CustomerCredential { pub role: String, pub password: String, pub valid_until: String, pub database: String }`
  - `pub fn customer_role_name(project_id: &str) -> Option<String>`
  - `pub fn random_customer_password() -> String`
  - `pub fn valid_until_utc(unix_seconds: u64) -> String`
  - `pub fn issue_customer_credential(project_id: &str, database: &str, password: &str, now_unix: u64) -> Option<CustomerCredential>`
  - `pub fn customer_door_accepts(role: &str) -> bool`
  - `pub fn credential_opens_project(role: &str, project_id: &str) -> bool`
  - `pub fn valid_proxy_host(host: &str) -> bool`
  - `pub fn customer_uri(role: &str, password: &str, host: &str, port: u16, database: &str) -> Option<String>`
  - `pub fn publishable_customer_uri(uri: &str) -> bool`
  - `pub fn customer_role_statements(credential: &CustomerCredential) -> Option<Vec<String>>`
  - `pub fn role_already_exists(sqlstate: &str) -> bool`

- [ ] **Step 1: Write the failing test**

Add the dependency line to `control_plane/Cargo.toml` after `pem.workspace = true`:

```toml
rand.workspace = true
```

In `control_plane/src/bin/briven_control_api.rs`, immediately after the `catalog` module block, add:

```rust
#[path = "../briven_control_api/customer_credential.rs"]
mod customer_credential;
```

Create `control_plane/src/briven_control_api/customer_credential.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::{
        credential_opens_project, customer_door_accepts, customer_role_name,
        customer_role_statements, customer_uri, issue_customer_credential, publishable_customer_uri,
        role_already_exists, valid_proxy_host, valid_until_utc, CUSTOMER_KEY_SECONDS,
    };

    const PASSWORD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const PROJECT: &str = "a1b2c3d4e5f67890";

    #[test]
    fn customer_credential_lasts_fifteen_minutes() {
        assert_eq!(CUSTOMER_KEY_SECONDS, 900);
        assert_eq!(valid_until_utc(0), "1970-01-01 00:00:00+00");
        assert_eq!(valid_until_utc(900), "1970-01-01 00:15:00+00");
        let issued = issue_customer_credential(PROJECT, "postgres", PASSWORD, 1_700_000_000)
            .expect("issued");
        assert_eq!(issued.role, "briven_a1b2c3d4e5f67890");
        assert_eq!(issued.password, PASSWORD);
        assert_eq!(issued.database, "postgres");
        assert_eq!(issued.valid_until, valid_until_utc(1_700_000_000 + 900));
        let uri = customer_uri(&issued.role, &issued.password, "db.internal", 5432, "postgres")
            .expect("uri");
        assert_eq!(
            uri,
            "postgresql://briven_a1b2c3d4e5f67890:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@db.internal:5432/postgres?sslmode=require"
        );
        assert!(publishable_customer_uri(&uri));
    }

    #[test]
    fn customer_credential_refuses_admin_roles() {
        for role in ["cloud_admin", "postgres", "neon_superuser", "briven_admin", "briven_ab", "BRIVEN_a1b2c3d4", "briven_a1b2c3d4-e", "briven_ABCDEFGH"] {
            assert!(!customer_door_accepts(role), "accepted {role}");
        }
        assert!(customer_door_accepts("briven_a1b2c3d4e5f67890"));
        assert!(customer_role_name("../x").is_none());
        assert_eq!(customer_role_name("A1B2-C3D4-E5F6-7890").as_deref(), Some("briven_a1b2c3d4e5f67890"));
        assert!(customer_role_name("short").is_none());
        assert!(customer_uri("cloud_admin", PASSWORD, "db.internal", 5432, "postgres").is_none());
        assert!(!publishable_customer_uri("postgresql://cloud_admin:secret@127.0.0.1:5432/postgres?sslmode=disable"));
        assert!(!valid_proxy_host("db.internal/evil"));
        assert!(!valid_proxy_host("user@db.internal"));
        assert!(valid_proxy_host("db.internal"));
        let issued = issue_customer_credential(PROJECT, "postgres", "not-hex", 0);
        assert!(issued.is_none());
        assert!(!role_already_exists("28000"));
        assert!(role_already_exists("42710"));
    }

    #[test]
    fn customer_credential_does_not_open_another_project() {
        let issued = issue_customer_credential(PROJECT, "postgres", PASSWORD, 0).expect("issued");
        assert!(credential_opens_project(&issued.role, PROJECT));
        assert!(!credential_opens_project(&issued.role, "bbbbbbbbbbbbbbbb"));
        assert!(!credential_opens_project("cloud_admin", PROJECT));
        let statements = customer_role_statements(&issued).expect("sql");
        assert!(statements.iter().all(|statement| !statement.contains("cloud_admin")));
        assert!(statements.iter().any(|statement| statement.contains("VALID UNTIL '1970-01-01 00:15:00+00'")));
        assert!(statements.iter().any(|statement| statement.contains("GRANT CONNECT ON DATABASE postgres TO briven_a1b2c3d4e5f67890")));
        let mut quoted = issued.clone();
        quoted.password = "aa'; drop role postgres; --".to_string();
        // pad check: a non-hex password must produce no SQL
        assert!(customer_role_statements(&quoted).is_none());
    }
}
```

The third test clones `CustomerCredential`, so derive `Clone` on that struct in Step 3.

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p control_plane --bin briven_control_api -- customer_credential -- --test-threads=8
```

Expected: FAIL to compile, with `cannot find function` (or `unresolved import`) for `customer_role_name`.

- [ ] **Step 3: Write the minimal implementation**

Replace `control_plane/src/briven_control_api/customer_credential.rs` with the tests from Step 1 plus this code above the test module:

```rust
use rand::Rng;

pub const CUSTOMER_KEY_SECONDS: u64 = 900;

#[derive(Clone)]
pub struct CustomerCredential {
    pub role: String,
    pub password: String,
    pub valid_until: String,
    pub database: String,
}

const FORBIDDEN_ROLES: &[&str] = &["cloud_admin", "postgres", "neon_superuser", "briven_admin"];

pub fn customer_role_name(project_id: &str) -> Option<String> {
    let body: String = project_id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_lowercase())
        .collect();
    if !(8..=48).contains(&body.len()) {
        return None;
    }
    let role = format!("briven_{body}");
    if !customer_door_accepts(&role) {
        return None;
    }
    Some(role)
}

pub fn random_customer_password() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn valid_until_utc(unix_seconds: u64) -> String {
    let days = (unix_seconds / 86_400) as i64;
    let seconds = unix_seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds / 3_600;
    let minute = (seconds % 3_600) / 60;
    let second = seconds % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}+00")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year += 1;
    }
    (year, month as u32, day as u32)
}

pub fn issue_customer_credential(
    project_id: &str,
    database: &str,
    password: &str,
    now_unix: u64,
) -> Option<CustomerCredential> {
    let role = customer_role_name(project_id)?;
    if !is_hex64(password) || !safe_ident(database) {
        return None;
    }
    Some(CustomerCredential {
        role,
        password: password.to_string(),
        valid_until: valid_until_utc(now_unix.saturating_add(CUSTOMER_KEY_SECONDS)),
        database: database.to_string(),
    })
}

pub fn customer_door_accepts(role: &str) -> bool {
    if FORBIDDEN_ROLES.contains(&role) {
        return false;
    }
    let Some(body) = role.strip_prefix("briven_") else {
        return false;
    };
    (8..=48).contains(&body.len())
        && body.chars().all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        && role.len() <= 63
}

pub fn credential_opens_project(role: &str, project_id: &str) -> bool {
    customer_role_name(project_id).as_deref() == Some(role) && customer_door_accepts(role)
}

pub fn valid_proxy_host(host: &str) -> bool {
    let bytes = host.as_bytes();
    (1..=253).contains(&bytes.len())
        && !host.contains("..")
        && bytes.iter().all(|byte| byte.is_ascii_alphanumeric() || *byte == b'.' || *byte == b'-')
        && bytes.first().is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes.last().is_some_and(|byte| byte.is_ascii_alphanumeric())
}

pub fn customer_uri(
    role: &str,
    password: &str,
    host: &str,
    port: u16,
    database: &str,
) -> Option<String> {
    if !customer_door_accepts(role) || !is_hex64(password) || !valid_proxy_host(host) || !safe_ident(database) {
        return None;
    }
    Some(format!(
        "postgresql://{role}:{password}@{host}:{port}/{database}?sslmode=require"
    ))
}

pub fn publishable_customer_uri(uri: &str) -> bool {
    !uri.contains("cloud_admin")
        && !uri.contains("sslmode=disable")
        && uri.contains("sslmode=require")
        && uri.starts_with("postgresql://briven_")
}

pub fn customer_role_statements(credential: &CustomerCredential) -> Option<Vec<String>> {
    if !customer_door_accepts(&credential.role) || !is_hex64(&credential.password) || !safe_ident(&credential.database) {
        return None;
    }
    if !credential.valid_until.chars().all(|character| {
        character.is_ascii_digit() || matches!(character, '-' | ' ' | ':' | '+')
    }) {
        return None;
    }
    let role = &credential.role;
    let database = &credential.database;
    let password = &credential.password;
    let until = &credential.valid_until;
    Some(vec![
        format!("CREATE ROLE {role} WITH LOGIN PASSWORD '{password}' VALID UNTIL '{until}'"),
        format!("ALTER ROLE {role} WITH LOGIN PASSWORD '{password}' VALID UNTIL '{until}'"),
        format!("GRANT CONNECT ON DATABASE {database} TO {role}"),
        format!("GRANT ALL PRIVILEGES ON DATABASE {database} TO {role}"),
        format!("GRANT ALL ON SCHEMA public TO {role}"),
    ])
}

pub fn role_already_exists(sqlstate: &str) -> bool {
    sqlstate == "42710"
}

fn is_hex64(password: &str) -> bool {
    password.len() == 64 && password.chars().all(|character| character.is_ascii_hexdigit())
}

fn safe_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }
    (1..=63).contains(&name.len())
        && name.chars().all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_')
}
```

After `safe_ident`, paste the `#[cfg(test)] mod tests` block from Step 1 unchanged, including its `use super::{...}` list. Step 1 already asserts `customer_role_name("../x").is_none()` and that `A1B2-C3D4-E5F6-7890` becomes `briven_a1b2c3d4e5f67890`. `../x` sanitizes to `x`, which is shorter than 8 letters, so the name is refused. `briven_ABCDEFGH` is refused because the letters after `briven_` must be lowercase.

`is_ascii_hexdigit` accepts `A-F`. `random_customer_password` only emits `0-9a-f`. The fixed test password is lowercase. In `is_hex64`, require lowercase hex so a mixed-case password cannot reach SQL:

```rust
fn is_hex64(password: &str) -> bool {
    password.len() == 64
        && password.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run:

```bash
cargo test -p control_plane --bin briven_control_api -- customer_credential -- --test-threads=8
```

Expected: PASS. `customer_credential_lasts_fifteen_minutes`, `customer_credential_refuses_admin_roles`, and `customer_credential_does_not_open_another_project` all pass. `test result: ok`.

- [ ] **Step 5: Commit**

```bash
git add control_plane/Cargo.toml control_plane/src/briven_control_api/customer_credential.rs control_plane/src/bin/briven_control_api.rs
git commit -m "$(cat <<'EOF'
feat(control): Define the 15-minute project key

The customer door names a role for one project, stamps it for 15
minutes, and refuses the engine master login. Unit tests do not open
a database cluster.
EOF
)"
```

Commit only those three paths. Do not push `main`.

---

### Task 2: Customer door decision

**Files:**
- Modify: `control_plane/src/briven_control_api/customer_credential.rs`
- Modify: `control_plane/src/bin/briven_control_api.rs` (`Connection` around line 69, `get_connection` around line 616)
- Test: `customer_connection_gate` tests in `customer_credential.rs`

**Interfaces:**
- Consumes: Task 1 functions `customer_uri`, `publishable_customer_uri`, `issue_customer_credential`, `customer_role_statements`, `credential_opens_project`, `valid_proxy_host`, `random_customer_password`, `role_already_exists`.
- Produces:
  - `pub enum ConnectionGate { Forbidden, Unavailable, NotReady, Issue }`
  - `pub fn connection_gate(customer_mode: bool, can_write: bool, proxy_host_set: bool, ready: bool) -> ConnectionGate`
  - Handler behavior: customer mode authorizes with `write=true`; missing or invalid proxy host returns 503 after that check; a project that is not `Ready` returns 409; `Issue` applies SQL with `cloud_admin` internally and returns only the customer URI.

- [ ] **Step 1: Write the failing test**

Append inside the existing `tests` module:

```rust
#[test]
fn customer_connection_gate_refuses_viewers_before_a_missing_proxy() {
    use super::{connection_gate, ConnectionGate};
    assert_eq!(
        connection_gate(true, false, false, true),
        ConnectionGate::Forbidden
    );
}

#[test]
fn customer_connection_gate_stays_unavailable_until_the_proxy_host_is_set() {
    use super::{connection_gate, ConnectionGate};
    assert_eq!(
        connection_gate(true, true, false, true),
        ConnectionGate::Unavailable
    );
    assert_eq!(
        connection_gate(true, true, true, false),
        ConnectionGate::NotReady
    );
}

#[test]
fn customer_connection_gate_issues_for_a_ready_writer_and_keeps_dev_mode() {
    use super::{connection_gate, ConnectionGate};
    assert_eq!(
        connection_gate(true, true, true, true),
        ConnectionGate::Issue
    );
    assert_eq!(
        connection_gate(false, false, false, false),
        ConnectionGate::Issue
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p control_plane --bin briven_control_api -- customer_connection_gate -- --test-threads=8
```

Expected: FAIL to compile, `cannot find function connection_gate`.

- [ ] **Step 3: Write the minimal implementation**

Add this above the test module in `customer_credential.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionGate {
    Forbidden,
    Unavailable,
    NotReady,
    Issue,
}

pub fn connection_gate(
    customer_mode: bool,
    can_write: bool,
    proxy_host_set: bool,
    ready: bool,
) -> ConnectionGate {
    if !customer_mode {
        return ConnectionGate::Issue;
    }
    if !can_write {
        return ConnectionGate::Forbidden;
    }
    if !proxy_host_set {
        return ConnectionGate::Unavailable;
    }
    if !ready {
        return ConnectionGate::NotReady;
    }
    ConnectionGate::Issue
}
```

In `briven_control_api.rs`, change `Connection` to:

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Connection {
    uri: String,
    environment: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<String>,
}
```

Add `use tokio_postgres::error::SqlState;` next to the existing `use tokio_postgres::NoTls;`.

Add this helper next to `enable_vector`. It must not format the admin URI or the SQL into the client error or the log line:

```rust
async fn apply_customer_role(admin_uri: &str, statements: &[String]) -> ApiResult<()> {
    let result = tokio::time::timeout(Duration::from_secs(15), async {
        let (client, connection) =
            tokio_postgres::connect(&format!("{admin_uri}?sslmode=disable"), NoTls).await?;
        let task = tokio::spawn(async move {
            let _ = connection.await;
        });
        let mut first = true;
        let mut outcome = Ok(());
        for statement in statements {
            match client.simple_query(statement).await {
                Ok(_) => {}
                Err(error) if first && error.code() == Some(&SqlState::DUPLICATE_OBJECT) => {}
                Err(error) => {
                    outcome = Err(error);
                    break;
                }
            }
            first = false;
        }
        task.abort();
        outcome
    })
    .await;
    match result {
        Ok(Ok(())) => Ok(()),
        _ => {
            eprintln!("Briven control API: customer role apply failed");
            Err(ApiError(
                StatusCode::BAD_GATEWAY,
                "local database engine operation failed",
            ))
        }
    }
}
```

`role_already_exists("42710")` stays the documented code for duplicate role. `SqlState::DUPLICATE_OBJECT` is that same code. The helper uses the typed code so a typo in a string cannot skip a real failure.

Replace `get_connection` with:

```rust
async fn get_connection(
    State(state): State<AppState>,
    headers: HeaderMap,
    RoutePath((id, branch)): RoutePath<(String, String)>,
) -> ApiResult<Json<Connection>> {
    if state.auth.uses_customer_identity() {
        let principal = authorize(&headers, &state.auth, true)?;
        let host = std::env::var("BRIVEN_CUSTOMER_PROXY_HOST").unwrap_or_default();
        let port = std::env::var("BRIVEN_CUSTOMER_PROXY_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(5432);
        let proxy_host_set = customer_credential::valid_proxy_host(&host) && port != 0;
        if !proxy_host_set {
            return Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "customer database credentials are not available yet",
            ));
        }
        let organization = principal.organization.as_str();
        let _guard = state.gate.lock().await;
        let projects = state.catalog.load(organization).await.map_err(internal)?;
        let found = project(&projects, &id)?;
        if found.state != ProjectState::Ready {
            return Err(ApiError(StatusCode::CONFLICT, "project is not ready"));
        }
        let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
        let tenant_id = id.parse::<TenantId>().map_err(internal)?;
        let timeline_id = env
            .get_branch_timeline_id(&branch, tenant_id)
            .ok_or(ApiError(StatusCode::NOT_FOUND, "branch not found"))?;
        let plane = ComputeControlPlane::load(env).map_err(engine)?;
        let endpoint = plane
            .endpoints
            .values()
            .find(|endpoint| {
                endpoint.tenant_id == tenant_id
                    && endpoint.timeline_id == timeline_id
                    && endpoint.status() == EndpointStatus::Running
            })
            .ok_or(ApiError(
                StatusCode::CONFLICT,
                "branch has no running compute",
            ))?;
        let admin_uri = endpoint.connstr("cloud_admin", "postgres");
        if !can_query(&admin_uri).await {
            return Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "branch compute is unavailable",
            ));
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(internal)?
            .as_secs();
        let password = customer_credential::random_customer_password();
        let credential = customer_credential::issue_customer_credential(&id, "postgres", &password, now)
            .ok_or(ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "control API operation failed",
            ))?;
        if !customer_credential::credential_opens_project(&credential.role, &id) {
            return Err(ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "control API operation failed",
            ));
        }
        let statements = customer_credential::customer_role_statements(&credential).ok_or(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "control API operation failed",
        ))?;
        apply_customer_role(&admin_uri, &statements).await?;
        let uri = customer_credential::customer_uri(
            &credential.role,
            &credential.password,
            &host,
            port,
            "postgres",
        )
        .filter(|uri| customer_credential::publishable_customer_uri(uri))
        .ok_or(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "control API operation failed",
        ))?;
        return Ok(Json(Connection {
            uri,
            environment: "customer",
            expires_at: Some(credential.valid_until),
        }));
    }

    let principal = authorize(&headers, &state.auth, false)?;
    let organization = principal.organization.as_str();
    let _guard = state.gate.lock().await;
    let projects = state.catalog.load(organization).await.map_err(internal)?;
    project(&projects, &id)?;
    let env = LocalEnv::load_config(&state.repo).map_err(engine)?;
    let tenant_id = id.parse::<TenantId>().map_err(internal)?;
    let timeline_id = env
        .get_branch_timeline_id(&branch, tenant_id)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "branch not found"))?;
    let plane = ComputeControlPlane::load(env).map_err(engine)?;
    let endpoint = plane
        .endpoints
        .values()
        .find(|endpoint| {
            endpoint.tenant_id == tenant_id
                && endpoint.timeline_id == timeline_id
                && endpoint.status() == EndpointStatus::Running
        })
        .ok_or(ApiError(
            StatusCode::CONFLICT,
            "branch has no running compute",
        ))?;
    if !can_query(&endpoint.connstr("cloud_admin", "postgres")).await {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "branch compute is unavailable",
        ));
    }
    Ok(Json(Connection {
        uri: endpoint.connstr("cloud_admin", "postgres"),
        environment: "local",
        expires_at: None,
    }))
}
```

The dev branch is unchanged except for `expires_at: None`, which serde omits. Customer mode never places `cloud_admin` in `Connection.uri`. `enable_vector` and `existing_name` stay as they are. Ready still means the existing create path already ran the vector query.

`authorize(..., true)` runs before the proxy-host check. A look-only person gets 403 `insufficient organization role` and never reaches the 503. When the proxy host is missing, the handler returns 503 and does not load the catalog. `principal` is used for the catalog load only after the proxy host is valid.

- [ ] **Step 4: Run the test to verify it passes**

Run:

```bash
cargo test -p control_plane --bin briven_control_api -- customer_ -- --test-threads=8
```

Expected: PASS for every `customer_credential_*` and `customer_connection_gate_*` test.

Then run the whole binary test set so the older name rules still pass:

```bash
cargo test -p control_plane --bin briven_control_api -- --test-threads=8
```

Expected: PASS, including `names_cannot_escape_cli_or_collide_with_paths` and `retrying_project_creation_reuses_one_tenant`. `test result: ok`. This command does not start a cluster and does not prove a live restart.

- [ ] **Step 5: Commit**

```bash
git add control_plane/src/briven_control_api/customer_credential.rs control_plane/src/bin/briven_control_api.rs
git commit -m "$(cat <<'EOF'
feat(control): Refuse a customer key until the private door can issue one

Look-only callers are rejected before the missing-proxy answer. Customer
mode never returns the engine master login.
EOF
)"
```

---

### Task 3: Website decisions for Create a project

**Files:**
- Create: `apps/api/src/services/engine-project.ts` in the website checkout
- Test: `apps/api/src/services/engine-project.test.ts`
- Working directory: `/tmp/briven-website-sprint3` on branch `sprint3-serverless-postgres`. Clone `flndrn-dev/briven-website` to that path and check out the branch if the folder is gone. Never commit on website `main`.

**Interfaces:**
- Consumes: nothing from the engine tasks. The HTTP shape it interprets is the engine's existing create answer: 201 created, 200 resumed, 409 conflict, body field `state` equals `ready` only after the engine's vector check.
- Produces, all exported from `engine-project.ts`:
  - `engineProjectName(slug: string | undefined, displayName: string): string | null`
  - `provisionPath(url: string | undefined, secret: string | undefined): 'engine' | 'shared-admin'`
  - `canCreateProject(role: string): boolean`
  - `interpretEngineCreate(status: number, state: string | undefined): 'created' | 'resumed' | 'conflict' | 'failed'`
  - `shouldKeepWebsiteRow(outcome: 'created' | 'resumed' | 'conflict' | 'failed'): boolean`
  - `controlRouteAllowed(method: string, path: string, role: string): boolean`
  - `controlKeyStillOpens(key: { revokedAt: Date | null; expiresAt: Date | null; orgDeleted: boolean } | null, now: number): boolean`
  - `signHallPass(secret: string, claims: { orgId: string; role: string; subject: string }): Promise<string>`
  - `requestEngineProject(fetchImpl: typeof fetch, input: { controlUrl: string; assertion: string; name: string }): Promise<{ status: number; body: { id?: string; state?: string } }>`
  - `class EngineProjectNameTaken`
  - `class EngineProjectFailed`

- [ ] **Step 1: Write the failing test**

Create `apps/api/src/services/engine-project.test.ts`:

```ts
import { describe, expect, it } from 'bun:test';

describe('engine-project', () => {
  it('uses a stable engine name and rejects a digit slug', async () => {
    const { engineProjectName } = await import('./engine-project.js');
    expect(engineProjectName('my-app', 'Ignored')).toBe('my-app');
    expect(engineProjectName(undefined, 'My App')).toBe('my-app');
    expect(engineProjectName(undefined, 'My App')).toBe('my-app');
    expect(engineProjectName('1abc', 'My App')).toBeNull();
    expect(engineProjectName(undefined, '1 App')).toBe('p1-app');
    expect(engineProjectName('', 'My App')).toBe('my-app');
  });

  it('keeps the website row only when the engine is ready', async () => {
    const { interpretEngineCreate, shouldKeepWebsiteRow, provisionPath, canCreateProject } =
      await import('./engine-project.js');
    expect(interpretEngineCreate(201, 'ready')).toBe('created');
    expect(interpretEngineCreate(200, 'ready')).toBe('resumed');
    expect(interpretEngineCreate(200, 'provisioning')).toBe('failed');
    expect(interpretEngineCreate(409, 'ready')).toBe('conflict');
    expect(interpretEngineCreate(500, 'failed')).toBe('failed');
    expect(shouldKeepWebsiteRow('created')).toBe(true);
    expect(shouldKeepWebsiteRow('resumed')).toBe(true);
    expect(shouldKeepWebsiteRow('conflict')).toBe(false);
    expect(shouldKeepWebsiteRow('failed')).toBe(false);
    expect(provisionPath('http://127.0.0.1:8787', 'a'.repeat(32))).toBe('engine');
    expect(provisionPath(undefined, undefined)).toBe('shared-admin');
    expect(provisionPath('http://127.0.0.1:8787', undefined)).toBe('shared-admin');
    expect(canCreateProject('owner')).toBe(true);
    expect(canCreateProject('admin')).toBe(true);
    expect(canCreateProject('developer')).toBe(true);
    expect(canCreateProject('viewer')).toBe(false);
  });

  it('refuses a look-only read of the connection door', async () => {
    const { controlRouteAllowed, controlKeyStillOpens } = await import('./engine-project.js');
    const connection = '/projects/abc/branches/main/connection';
    expect(controlRouteAllowed('GET', connection, 'developer')).toBe(true);
    expect(controlRouteAllowed('GET', connection, 'viewer')).toBe(false);
    expect(controlRouteAllowed('GET', '/projects/abc', 'viewer')).toBe(true);
    expect(controlRouteAllowed('POST', '/projects', 'viewer')).toBe(false);
    expect(controlRouteAllowed('DELETE', '/projects/abc', 'owner')).toBe(false);
    const now = Date.parse('2026-09-29T00:00:00Z');
    expect(controlKeyStillOpens({ revokedAt: null, expiresAt: null, orgDeleted: false }, now)).toBe(true);
    expect(controlKeyStillOpens({ revokedAt: new Date(now), expiresAt: null, orgDeleted: false }, now)).toBe(false);
    expect(controlKeyStillOpens({ revokedAt: null, expiresAt: new Date(now), orgDeleted: false }, now)).toBe(false);
    expect(controlKeyStillOpens({ revokedAt: null, expiresAt: null, orgDeleted: true }, now)).toBe(false);
    expect(controlKeyStillOpens(null, now)).toBe(false);
  });

  it('signs a hall pass for exactly 60 seconds', async () => {
    const { signHallPass, requestEngineProject } = await import('./engine-project.js');
    const { decodeJwt } = await import('jose');
    const token = await signHallPass('a'.repeat(32), {
      orgId: 'org_1',
      role: 'developer',
      subject: 'user_1',
    });
    const payload = decodeJwt(token);
    expect(payload.iss).toBe('briven-api');
    expect(payload.aud).toBe('briven-control');
    expect(payload.sub).toBe('user_1');
    expect(payload.org_id).toBe('org_1');
    expect(payload.role).toBe('developer');
    expect(payload.exp! - payload.iat!).toBe(60);

    const seen: { url?: string; authorization?: string; body?: string } = {};
    const response = await requestEngineProject(
      (async (url: string | URL | Request, init?: RequestInit) => {
        seen.url = String(url);
        seen.authorization = new Headers(init?.headers).get('authorization') ?? undefined;
        seen.body = String(init?.body);
        return new Response(JSON.stringify({ id: 'tenant-1', state: 'ready' }), { status: 201 });
      }) as typeof fetch,
      { controlUrl: 'http://127.0.0.1:8787', assertion: 'hall-pass', name: 'my-app' },
    );
    expect(response.status).toBe(201);
    expect(response.body).toEqual({ id: 'tenant-1', state: 'ready' });
    expect(seen.url).toBe('http://127.0.0.1:8787/v1/projects');
    expect(seen.authorization).toBe('Bearer hall-pass');
    expect(seen.body).toBe('{"name":"my-app"}');
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cd /tmp/briven-website-sprint3/apps/api && bun test src/services/engine-project.test.ts
```

Expected: FAIL because `./engine-project.js` cannot be found.

- [ ] **Step 3: Write the minimal implementation**

Create `apps/api/src/services/engine-project.ts`:

```ts
import { SignJWT } from 'jose';

const ENGINE_NAME = /^[a-z](?:[a-z0-9-]{0,30}[a-z0-9])?$/;

export function engineProjectName(slug: string | undefined, displayName: string): string | null {
  if (slug !== undefined && slug !== '') {
    return ENGINE_NAME.test(slug) ? slug : null;
  }
  let derived = displayName
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
  if (/^[0-9]/.test(derived)) derived = `p${derived}`;
  if (derived.length > 32) derived = derived.slice(0, 32).replace(/-+$/g, '');
  return ENGINE_NAME.test(derived) ? derived : null;
}

export function provisionPath(
  url: string | undefined,
  secret: string | undefined,
): 'engine' | 'shared-admin' {
  return url && secret ? 'engine' : 'shared-admin';
}

export function canCreateProject(role: string): boolean {
  return role === 'owner' || role === 'admin' || role === 'developer';
}

export type EngineCreateOutcome = 'created' | 'resumed' | 'conflict' | 'failed';

export function interpretEngineCreate(status: number, state: string | undefined): EngineCreateOutcome {
  if (status === 409) return 'conflict';
  if (state === 'ready' && status === 201) return 'created';
  if (state === 'ready' && status === 200) return 'resumed';
  return 'failed';
}

export function shouldKeepWebsiteRow(outcome: EngineCreateOutcome): boolean {
  return outcome === 'created' || outcome === 'resumed';
}

const CONTROL_ROUTES = [
  /^\/projects$/,
  /^\/projects\/[a-z0-9-]+$/,
  /^\/projects\/[a-z0-9-]+\/branches$/,
  /^\/projects\/[a-z0-9-]+\/computes$/,
  /^\/projects\/[a-z0-9-]+\/branches\/[a-z0-9-]+\/computes$/,
  /^\/projects\/[a-z0-9-]+\/branches\/[a-z0-9-]+\/connection$/,
];

export function controlRouteAllowed(method: string, path: string, role: string): boolean {
  if (method !== 'GET' && method !== 'POST') return false;
  if (!CONTROL_ROUTES.some((route) => route.test(path))) return false;
  const connection = /^\/projects\/[a-z0-9-]+\/branches\/[a-z0-9-]+\/connection$/;
  if (role === 'viewer' && (method !== 'GET' || connection.test(path))) return false;
  return true;
}

export function controlKeyStillOpens(
  key: { revokedAt: Date | null; expiresAt: Date | null; orgDeleted: boolean } | null,
  now: number,
): boolean {
  if (!key || key.orgDeleted || key.revokedAt) return false;
  if (key.expiresAt && key.expiresAt.getTime() <= now) return false;
  return true;
}

export async function signHallPass(
  secret: string,
  claims: { orgId: string; role: string; subject: string },
): Promise<string> {
  return new SignJWT({ org_id: claims.orgId, role: claims.role })
    .setProtectedHeader({ alg: 'HS256' })
    .setIssuer('briven-api')
    .setAudience('briven-control')
    .setSubject(claims.subject)
    .setIssuedAt()
    .setExpirationTime('60s')
    .sign(new TextEncoder().encode(secret));
}

export interface EngineProjectBody {
  id?: string;
  state?: string;
}

export async function requestEngineProject(
  fetchImpl: typeof fetch,
  input: { controlUrl: string; assertion: string; name: string },
): Promise<{ status: number; body: EngineProjectBody }> {
  const target = new URL('/v1/projects', input.controlUrl);
  const response = await fetchImpl(target, {
    method: 'POST',
    headers: {
      authorization: `Bearer ${input.assertion}`,
      'content-type': 'application/json',
    },
    body: JSON.stringify({ name: input.name }),
    signal: AbortSignal.timeout(120_000),
  });
  const body = (await response.json().catch(() => ({}))) as EngineProjectBody;
  return { status: response.status, body };
}

export class EngineProjectNameTaken extends Error {
  readonly code = 'engine_project_name_taken';
  constructor() {
    super('project name already exists');
    this.name = 'EngineProjectNameTaken';
  }
}

export class EngineProjectFailed extends Error {
  readonly code = 'engine_project_failed';
  constructor() {
    super('customer database was not ready');
    this.name = 'EngineProjectFailed';
  }
}
```

`engineProjectName(undefined, '1 App')` slugifies to `1-app`, then prefixes `p`, producing `p1-app`. That derived prefix is only for a blank slug. A typed slug `1abc` stays `null`. Do not change `genSlug` in `projects.ts` in this task.

`controlKeyStillOpens` pins the same rule `resolveControlKey` already applies (revoked, expired, or deleted organization does not open). Do not rewrite `resolveControlKey`.

- [ ] **Step 4: Run the test to verify it passes**

Run:

```bash
cd /tmp/briven-website-sprint3/apps/api && bun test src/services/engine-project.test.ts
```

Expected: 4 pass, 0 fail. Do not treat a full-website `tsc --noEmit` failure as a failure of this task. That command already reports older errors outside these files.

- [ ] **Step 5: Commit**

From `/tmp/briven-website-sprint3`:

```bash
git add apps/api/src/services/engine-project.ts apps/api/src/services/engine-project.test.ts
git commit -m "$(cat <<'EOF'
feat(api): Decide when Create a project may ask the engine

The engine name stays stable across a retry. A look-only person cannot
create, and cannot read the customer connection door.
EOF
)"
```

Do not push website `main`. A push of this branch, when asked, is `git push origin HEAD:sprint3-serverless-postgres` only.

---

### Task 4: Wire Create a project to one engine tenant

**Files:**
- Modify: `apps/api/src/db/schema.ts` (projects table, after `dataSchemaName`)
- Create: `apps/api/drizzle/migrations/0058_engine_project_id.sql`
- Modify: `apps/api/drizzle/migrations/meta/_journal.json` (append idx 44 after idx 43 `0057_org_control_keys`)
- Modify: `apps/api/src/services/orgs.ts` (next to `isOrgMember`)
- Modify: `apps/api/src/services/projects.ts` (`createProject`)
- Modify: `apps/api/src/routes/projects.ts` (POST `/v1/projects`)
- Modify: `apps/api/src/routes/engine-control.ts` (`allowedRoutes` check and the viewer check)
- Test: the Task 3 file still passes. This task does not add a test that needs PostgreSQL.

**Interfaces:**
- Consumes: Task 3 exports `engineProjectName`, `provisionPath`, `canCreateProject`, `interpretEngineCreate`, `shouldKeepWebsiteRow`, `controlRouteAllowed`, `signHallPass`, `requestEngineProject`, `EngineProjectNameTaken`, `EngineProjectFailed`.
- Produces: nullable `projects.engine_project_id`; `orgRoleFor(userId: string, orgId: string): Promise<OrgRole | null>`; Create a project calls the engine only when both settings are set; HTTP 403 for a viewer; HTTP 409 for `EngineProjectNameTaken`.

- [ ] **Step 1: Write the failing test**

The decisions are already tested in Task 3. This step adds one assertion to `engine-project.test.ts` inside the "stable engine name" test, before implementation of the column is irrelevant to that assertion. Add:

```ts
expect(engineProjectName(undefined, 'My App')).not.toBe(engineProjectName('other-app', 'My App'));
```

`engineProjectName('other-app', 'My App')` is `other-app`. The blank-slug call is `my-app`. They differ, so a typed slug is never replaced by the display name. Run the test. Expected: PASS already, because Task 3 implemented `engineProjectName`. If it fails, stop and fix Task 3 before touching schema.

The wiring below is then covered by those decisions. Do not point this task at a live engine.

- [ ] **Step 2: Confirm the decision test still passes**

Run:

```bash
cd /tmp/briven-website-sprint3/apps/api && bun test src/services/engine-project.test.ts
```

Expected: 4 pass, 0 fail.

- [ ] **Step 3: Write the schema, the migration, and the wiring**

In `apps/api/src/db/schema.ts`, inside `projects`, after `dataSchemaName`:

```ts
    engineProjectId: text('engine_project_id'),
```

Inside the projects index callback, next to `orgIdx`:

```ts
    engineProjectIdx: uniqueIndex('projects_engine_project_id_idx')
      .on(t.engineProjectId)
      .where(sql`"engine_project_id" is not null`),
```

`sql` is already imported in this file. The column stays nullable. Rows created on the practice path leave it empty.

Create `apps/api/drizzle/migrations/0058_engine_project_id.sql`:

```sql
ALTER TABLE "projects" ADD COLUMN IF NOT EXISTS "engine_project_id" text;
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "projects_engine_project_id_idx" ON "projects" ("engine_project_id") WHERE "engine_project_id" IS NOT NULL;
```

Append this object as the last `entries` item in `apps/api/drizzle/migrations/meta/_journal.json`. The previous last item is idx 43, tag `0057_org_control_keys`, when `1790600000000`. Keep that item, and add a comma after it:

```json
		{
			"idx": 44,
			"version": "7",
			"when": 1790700000000,
			"tag": "0058_engine_project_id",
			"breakpoints": true
		}
```

Do not run `drizzle-kit generate`. Do not edit snapshot JSON under `meta/` other than `_journal.json`.

In `apps/api/src/services/orgs.ts`, add this next to `isOrgMember`. `OrgRole` is already imported in that file from the schema:

```ts
export async function orgRoleFor(userId: string, orgId: string): Promise<OrgRole | null> {
  const db = getDb();
  const [row] = await db
    .select({ role: orgMembers.role })
    .from(orgMembers)
    .where(and(eq(orgMembers.userId, userId), eq(orgMembers.orgId, orgId)))
    .limit(1);
  return row?.role ?? null;
}
```

In `apps/api/src/services/projects.ts`, add these imports:

```ts
import { env } from '../env.js';
import {
  EngineProjectFailed,
  EngineProjectNameTaken,
  engineProjectName,
  interpretEngineCreate,
  provisionPath,
  requestEngineProject,
  shouldKeepWebsiteRow,
  signHallPass,
} from './engine-project.js';
import type { OrgRole } from '../db/schema.js';
```

`OrgRole` may already be available from the existing schema import. Import it once.

Add `orgRole?: OrgRole` to `CreateProjectInput`.

Add this helper in `projects.ts` above `createProject`:

```ts
async function rollbackWebsiteRows(projectId: string): Promise<void> {
  const db = getDb();
  try {
    await db.delete(projectMembers).where(eq(projectMembers.projectId, projectId));
  } catch (memberErr) {
    log.warn('project_rollback_member_delete_failed', {
      projectId,
      message: memberErr instanceof Error ? memberErr.message : String(memberErr),
    });
  }
  try {
    await db.delete(projects).where(eq(projects.id, projectId));
  } catch (rowErr) {
    log.warn('project_rollback_row_delete_failed', {
      projectId,
      message: rowErr instanceof Error ? rowErr.message : String(rowErr),
    });
  }
}
```

Replace `createProject` with:

```ts
export async function createProject(input: CreateProjectInput): Promise<Project> {
  const useEngine =
    provisionPath(env.BRIVEN_ENGINE_CONTROL_URL, env.BRIVEN_CONTROL_IDENTITY_SECRET) === 'engine';
  const engineName = useEngine ? engineProjectName(input.slug, input.name) : null;
  if (useEngine && !engineName) {
    throw new ValidationError('project name cannot be used for the customer database', {
      slug: input.slug ?? '',
    });
  }
  if (useEngine && !input.orgRole) {
    throw new ValidationError('organization role is required', {});
  }
  const slug = input.slug ?? genSlug();
  if (!isValidSlug(slug)) {
    throw new ValidationError('slug must be lowercase alphanumeric with hyphens, 1-32 chars', {
      slug,
    });
  }

  const tier = await getTierForOrg(input.orgId);
  await assertProjectCreateAllowed(input.orgId, tier);

  const projectId = newId('p');
  const row: NewProject = {
    id: projectId,
    slug,
    name: input.name,
    orgId: input.orgId,
    region: input.region ?? 'eu-west-1',
    tier,
    dataSchemaName: dbNameFor(projectId),
  };

  const db = getDb();
  const [created] = await db.insert(projects).values(row).returning();
  if (!created) throw new Error('project insert returned no row');

  await db.insert(projectMembers).values({
    projectId: created.id,
    userId: input.createdByUserId,
    role: 'owner',
  });

  try {
    if (!useEngine) {
      await provisionProjectDatabase(created.id);
    } else {
      const assertion = await signHallPass(env.BRIVEN_CONTROL_IDENTITY_SECRET!, {
        orgId: input.orgId,
        role: input.orgRole!,
        subject: input.createdByUserId,
      });
      let response: { status: number; body: { id?: string; state?: string } };
      try {
        response = await requestEngineProject(fetch, {
          controlUrl: env.BRIVEN_ENGINE_CONTROL_URL!,
          assertion,
          name: engineName!,
        });
      } catch (err) {
        log.error('project_engine_create_unreachable', {
          projectId: created.id,
          message: err instanceof Error ? err.message : String(err),
        });
        throw new EngineProjectFailed();
      }
      const outcome = interpretEngineCreate(response.status, response.body.state);
      if (outcome === 'conflict') throw new EngineProjectNameTaken();
      if (!shouldKeepWebsiteRow(outcome) || !response.body.id) throw new EngineProjectFailed();
      await db
        .update(projects)
        .set({ engineProjectId: response.body.id, updatedAt: new Date() })
        .where(eq(projects.id, created.id));
    }
  } catch (err) {
    if (!useEngine) {
      try {
        await dropProjectDatabase(created.id);
      } catch (dropErr) {
        log.warn('project_rollback_database_drop_failed', {
          projectId: created.id,
          message: dropErr instanceof Error ? dropErr.message : String(dropErr),
        });
      }
    }
    await rollbackWebsiteRows(created.id);
    throw err;
  }

  const [fresh] = await db.select().from(projects).where(eq(projects.id, created.id)).limit(1);
  if (!fresh) throw new Error('project insert returned no row');
  if (useEngine && !fresh.engineProjectId) {
    await rollbackWebsiteRows(fresh.id);
    throw new EngineProjectFailed();
  }

  try {
    await enableForProject(fresh.id, {
      id: input.createdByUserId,
      ipHash: null,
      userAgent: 'project-create-auto-enable',
    });
  } catch (err) {
    if (!(err instanceof McpPlanRequiredError)) {
      log.warn('project_mcp_auto_enable_failed', {
        projectId: fresh.id,
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }

  return fresh;
}
```

Behavior this function must keep:

- Both engine settings unset: call `provisionProjectDatabase`, and on failure drop that practice database and the website rows. `genSlug` still fills a blank slug. Do not call the engine.
- Both settings set: do not call `provisionProjectDatabase` or `dropProjectDatabase`. The engine name is `engineName`, not `genSlug()` and not the website project id (`p_` plus a ULID is not a legal engine name). A blank slug still gets a random website slug from `genSlug()`, so two teams can use the same display name without fighting the global slug unique index. The engine name comes from the display name, so a retry of "My App" asks for `my-app` again.
- A typed slug that starts with a digit throws `ValidationError` before the insert on the engine path.
- HTTP 409 from the engine throws `EngineProjectNameTaken`, deletes the new website rows, and does not take over the existing engine tenant.
- Any other engine outcome, including `state` other than `ready`, throws `EngineProjectFailed` and deletes the new website rows. The engine keeps the unfinished tenant so the next same-name create resumes it.
- Persist `engine_project_id` immediately after a ready response, before the MCP enable. MCP failure must not delete the project.
- The non-null assertion on the two env fields is safe only because `provisionPath` already required both. Do not read one without the other.

In `apps/api/src/routes/projects.ts`, import `canCreateProject` and `EngineProjectNameTaken` from `../services/engine-project.js`, and `orgRoleFor` from `../services/orgs.js`. Replace the membership block inside POST `/v1/projects` with:

```ts
  let targetOrgId: string;
  if (parsed.data.orgId) {
    targetOrgId = parsed.data.orgId;
  } else {
    const org = await getDefaultOrgForUser(user.id);
    targetOrgId = org.id;
  }
  const role = await orgRoleFor(user.id, targetOrgId);
  if (!role) {
    return c.json({ code: 'forbidden', message: 'not a member of that org' }, 403);
  }
  if (!canCreateProject(role)) {
    return c.json({ code: 'forbidden', message: 'insufficient organization role' }, 403);
  }
  let project;
  try {
    project = await createProject({
      name: parsed.data.name,
      orgId: targetOrgId,
      createdByUserId: user.id,
      slug: parsed.data.slug,
      region: parsed.data.region,
      orgRole: role,
    });
  } catch (err) {
    if (err instanceof EngineProjectNameTaken) {
      return c.json({ code: 'conflict', message: 'project name already exists' }, 409);
    }
    throw err;
  }
```

Keep the audit, `ensureDefaultProjectStorage`, and the 201 response that follow. `isOrgMember` is no longer used on this route. Leave `isOrgMember` exported for its other callers.

In `apps/api/src/routes/engine-control.ts`, import `controlPathExists` and `controlRouteAllowed` from `../services/engine-project.js`. Delete the `allowedRoutes` array.

Add this exported helper to `apps/api/src/services/engine-project.ts`, next to `controlRouteAllowed`:

```ts
export function controlPathExists(method: string, path: string): boolean {
  return controlRouteAllowed(method, path, 'owner');
}
```

`controlPathExists` only answers whether this door knows the path. It uses the owner allow-list for that answer, so a look-only person is not part of the path check.

Add this assertion to the route test in `engine-project.test.ts`:

```ts
expect(controlPathExists('GET', '/projects/abc/branches/main/connection')).toBe(true);
expect(controlPathExists('DELETE', '/projects/abc')).toBe(false);
```

Import `controlPathExists` in that test from `./engine-project.js`.

In the route, after reading `method` and `path`:

```ts
  if (!controlPathExists(method, path)) {
    return c.json({ code: 'not_found' }, 404);
  }
```

After `principal` is known, replace `if (method !== 'GET' && principal.role === 'viewer')` with:

```ts
  if (!controlRouteAllowed(method, path, principal.role)) {
    return c.json({ code: 'forbidden' }, 403);
  }
```

A look-only GET of `/projects/:id/branches/:branch/connection` is now 403. A look-only GET of `/projects/:id` stays allowed. The hall pass is still signed for 60 seconds by the existing `SignJWT` block in this file. Do not lengthen it.

- [ ] **Step 4: Run the test to verify it passes**

Run:

```bash
cd /tmp/briven-website-sprint3/apps/api && bun test src/services/engine-project.test.ts
```

Expected: 4 pass, 0 fail, including the new `controlPathExists` assertions.

Also run the existing projects route probe so the CSRF expectation still holds:

```bash
cd /tmp/briven-website-sprint3/apps/api && bun test src/routes/projects.test.ts
```

Expected: the file's existing tests pass. That file does not open a customer database.

- [ ] **Step 5: Commit**

From `/tmp/briven-website-sprint3`:

```bash
git add apps/api/src/db/schema.ts apps/api/drizzle/migrations/0058_engine_project_id.sql apps/api/drizzle/migrations/meta/_journal.json apps/api/src/services/orgs.ts apps/api/src/services/projects.ts apps/api/src/services/engine-project.ts apps/api/src/services/engine-project.test.ts apps/api/src/routes/projects.ts apps/api/src/routes/engine-control.ts
git commit -m "$(cat <<'EOF'
feat(api): Ask the engine for one empty database per project

Create a project uses the private engine when that door is configured.
A finished name is refused, and a retry keeps the same engine tenant.
EOF
)"
```

Do not push website `main`.

---

## Spec coverage

Checked against `docs/superpowers/specs/2026-09-29-customer-database-home-design.md`.

| Spec section | Where this plan covers it |
| --- | --- |
| Decision: databases live on `187.77.183.190` in a private back room | Global constraint. Not a task. The install is the unwritten second plan. |
| Rooms: website control database, one engine database per project, 60-second hall pass | Task 3 `signHallPass` and Task 4 create path. The running engine processes stay in the second plan. |
| Create a project steps 1–8, including resume and conflict | Task 4, using the engine `existing_name` rule already committed. Do not rewrite it. Ready still requires the existing vector query inside `ensure_main_compute`. |
| Customer key: random, 15 minutes, one database, TLS, master login refused | Task 1 and Task 2. |
| Look-only person, or a removed pass, cannot create and cannot get a write key | Task 3 `canCreateProject` and `controlKeyStillOpens`; Task 4 route and connection door. `resolveControlKey` already drops revoked, expired, and deleted-organization keys. |
| A key from one project fails on another | Task 1 `credential_opens_project`. The handler returns the key only when that check passes. The live proxy that enforces it on a changed address is the second plan. |
| Private test list on the briven.tech computer | Second plan. Do not add SSH steps here. |
| Saved copies stay off, no import, no public cutover, passwords out of git | Global constraints. No task turns those on. |

## Self-review

- Spec coverage is the table above. The live-computer install is a gap on purpose.
- Placeholder scan: no TBD, TODO, or "similar to task" steps. Task 1 Step 1 asserts `customer_role_name("../x")` is none and refuses `briven_ABCDEFGH`. Task 2 does not bind an unused principal. Task 4 checks the path with `controlPathExists` only.
- Types: `ConnectionGate` names match Task 2 tests. `EngineCreateOutcome` values match `interpretEngineCreate` and `shouldKeepWebsiteRow`. `engineProjectId` in schema matches SQL `engine_project_id`. `controlPathExists` is defined in Task 4 before the route uses it. `orgRole` on `CreateProjectInput` is the role Task 4 passes into `signHallPass`.
