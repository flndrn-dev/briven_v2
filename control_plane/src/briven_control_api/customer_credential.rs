use rand::Rng;

pub const CUSTOMER_KEY_SECONDS: u64 = 900;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceKind {
    Platform,
    Runtime,
}

impl ServiceKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "platform" => Some(Self::Platform),
            "runtime" => Some(Self::Runtime),
            _ => None,
        }
    }

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Platform => "platform",
            Self::Runtime => "runtime",
        }
    }
}

pub fn service_role_name(project_id: &str, kind: ServiceKind) -> Option<String> {
    let role = format!("{}_{}", customer_role_name(project_id)?, kind.suffix());
    (role.len() <= 63).then_some(role)
}

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

pub fn issue_service_credential(
    project_id: &str,
    kind: ServiceKind,
    database: &str,
    password: &str,
    now_unix: u64,
) -> Option<CustomerCredential> {
    let mut credential = issue_customer_credential(project_id, database, password, now_unix)?;
    credential.role = service_role_name(project_id, kind)?;
    Some(credential)
}

pub fn customer_door_accepts(role: &str) -> bool {
    if FORBIDDEN_ROLES.contains(&role) {
        return false;
    }
    let Some(body) = role.strip_prefix("briven_") else {
        return false;
    };
    let body = body
        .strip_suffix("_platform")
        .or_else(|| body.strip_suffix("_runtime"))
        .unwrap_or(body);
    (8..=48).contains(&body.len())
        && body
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        && role.len() <= 63
}

pub fn credential_opens_project(role: &str, project_id: &str) -> bool {
    customer_door_accepts(role)
        && (customer_role_name(project_id).as_deref() == Some(role)
            || service_role_name(project_id, ServiceKind::Platform).as_deref() == Some(role)
            || service_role_name(project_id, ServiceKind::Runtime).as_deref() == Some(role))
}

pub fn valid_proxy_host(host: &str) -> bool {
    let bytes = host.as_bytes();
    (1..=253).contains(&bytes.len())
        && !host.contains("..")
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'.' || *byte == b'-')
        && bytes
            .first()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes
            .last()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
}

pub fn customer_uri(
    role: &str,
    password: &str,
    host: &str,
    port: u16,
    database: &str,
) -> Option<String> {
    if !customer_door_accepts(role)
        || !is_hex64(password)
        || !valid_proxy_host(host)
        || !safe_ident(database)
    {
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

/// The inherited proxy needs an explicit endpoint when several customer
/// computes share one TLS hostname. PostgreSQL startup options carry that ID.
pub fn with_endpoint_hint(uri: &str, endpoint_id: &str) -> Option<String> {
    if !(4..=100).contains(&endpoint_id.len())
        || !endpoint_id.starts_with("ep-")
        || !endpoint_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return None;
    }
    let mut url = url::Url::parse(uri).ok()?;
    url.query_pairs_mut()
        .append_pair("options", &format!("endpoint={endpoint_id}"));
    Some(url.to_string())
}

pub fn customer_role_statements(credential: &CustomerCredential) -> Option<Vec<String>> {
    if !customer_door_accepts(&credential.role)
        || !is_hex64(&credential.password)
        || !safe_ident(&credential.database)
    {
        return None;
    }
    if !credential
        .valid_until
        .chars()
        .all(|character| character.is_ascii_digit() || matches!(character, '-' | ' ' | ':' | '+'))
    {
        return None;
    }
    let role = &credential.role;
    let database = &credential.database;
    let password = &credential.password;
    let until = &credential.valid_until;
    let customer = role
        .strip_suffix("_platform")
        .or_else(|| role.strip_suffix("_runtime"))
        .unwrap_or(role);
    let platform = format!("{customer}_platform");
    let runtime = format!("{customer}_runtime");
    if platform.len() > 63 {
        return None;
    }
    let roles = [customer, platform.as_str(), runtime.as_str()];
    let mut statements = Vec::new();
    for granted in roles {
        statements.push(format!("DO $briven$ BEGIN CREATE ROLE {granted} NOLOGIN; EXCEPTION WHEN duplicate_object THEN NULL; END $briven$"));
        statements.push(format!(
            "ALTER ROLE {granted} NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS"
        ));
    }
    statements.push(format!(
        "ALTER ROLE {role} WITH LOGIN PASSWORD '{password}' VALID UNTIL '{until}'"
    ));
    // All untrusted application sessions SET ROLE to the customer role, so
    // tables created by SQL, functions and schema deployments share an owner.
    // Only the platform role can reset to its metadata privileges.
    statements.push(format!("GRANT {customer} TO {runtime}, {platform}"));
    statements.push(format!("CREATE TABLE IF NOT EXISTS public._briven_migrations (id text PRIMARY KEY, deployment_id text, applied_at timestamptz NOT NULL DEFAULT now(), summary jsonb)"));
    statements.push("CREATE TABLE IF NOT EXISTS public._briven_meta (key text PRIMARY KEY, value jsonb NOT NULL)".to_string());
    for granted in roles {
        statements.push(format!("GRANT CONNECT ON DATABASE {database} TO {granted}"));
        statements.push(format!("GRANT USAGE, CREATE ON SCHEMA public TO {granted}"));
        statements.push(format!(
            "GRANT ALL ON ALL TABLES IN SCHEMA public TO {granted}"
        ));
        statements.push(format!(
            "GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO {granted}"
        ));
        statements.push(format!("ALTER DEFAULT PRIVILEGES FOR ROLE {customer} IN SCHEMA public GRANT ALL ON TABLES TO {granted}"));
        statements.push(format!("ALTER DEFAULT PRIVILEGES FOR ROLE {customer} IN SCHEMA public GRANT ALL ON SEQUENCES TO {granted}"));
    }
    statements.push(format!("REVOKE ALL ON TABLE public._briven_meta, public._briven_migrations FROM PUBLIC, {customer}, {runtime}"));
    statements.push(format!(
        "GRANT ALL ON TABLE public._briven_meta, public._briven_migrations TO {platform}"
    ));
    Some(statements)
}

pub fn role_already_exists(sqlstate: &str) -> bool {
    sqlstate == "42710"
}

fn is_hex64(password: &str) -> bool {
    password.len() == 64
        && password
            .chars()
            .all(|character| matches!(character, '0'..='9' | 'a'..='f'))
}

fn safe_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }
    (1..=63).contains(&name.len())
        && name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

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

#[cfg(test)]
mod tests {
    #[test]
    fn service_credentials_are_independent_and_bound_to_the_exact_tenant() {
        use super::*;
        for kind in [ServiceKind::Platform, ServiceKind::Runtime] {
            let credential =
                issue_service_credential(PROJECT, kind, "postgres", PASSWORD, 0).unwrap();
            assert_ne!(credential.role, customer_role_name(PROJECT).unwrap());
            assert!(credential_opens_project(&credential.role, PROJECT));
            assert!(!credential_opens_project(
                &credential.role,
                "bbbbbbbbbbbbbbbb"
            ));
            let sql = customer_role_statements(&credential).unwrap();
            assert_eq!(
                sql.iter()
                    .filter(|s| s.contains("WITH LOGIN PASSWORD"))
                    .count(),
                1
            );
            assert!(
                sql.iter()
                    .any(|s| s.starts_with(&format!("ALTER ROLE {} WITH LOGIN", credential.role)))
            );
            assert!(sql.iter().any(|s| s.contains("REVOKE ALL ON TABLE public._briven_meta, public._briven_migrations FROM PUBLIC, briven_a1b2c3d4e5f67890, briven_a1b2c3d4e5f67890_runtime")));
        }
        for role in [
            "briven_a1b2c3d4e5f67890_admin",
            "briven_a1b2c3d4e5f67890_runtime_platform",
            "briven_a1b2c3d4e5f67890_runtime_evil",
        ] {
            assert!(!customer_door_accepts(role));
            assert!(!credential_opens_project(role, PROJECT));
        }
    }

    use super::{
        CUSTOMER_KEY_SECONDS, credential_opens_project, customer_door_accepts, customer_role_name,
        customer_role_statements, customer_uri, issue_customer_credential,
        publishable_customer_uri, role_already_exists, valid_proxy_host, valid_until_utc,
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
        let uri = customer_uri(
            &issued.role,
            &issued.password,
            "db.internal",
            5432,
            "postgres",
        )
        .expect("uri");
        assert_eq!(
            uri,
            "postgresql://briven_a1b2c3d4e5f67890:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@db.internal:5432/postgres?sslmode=require"
        );
        assert!(publishable_customer_uri(&uri));
    }

    #[test]
    fn customer_credential_refuses_admin_roles() {
        for role in [
            "cloud_admin",
            "postgres",
            "neon_superuser",
            "briven_admin",
            "briven_ab",
            "BRIVEN_a1b2c3d4",
            "briven_a1b2c3d4-e",
            "briven_ABCDEFGH",
        ] {
            assert!(!customer_door_accepts(role), "accepted {role}");
        }
        assert!(customer_door_accepts("briven_a1b2c3d4e5f67890"));
        assert!(customer_role_name("../x").is_none());
        assert_eq!(
            customer_role_name("A1B2-C3D4-E5F6-7890").as_deref(),
            Some("briven_a1b2c3d4e5f67890")
        );
        assert!(customer_role_name("short").is_none());
        assert!(customer_uri("cloud_admin", PASSWORD, "db.internal", 5432, "postgres").is_none());
        assert!(!publishable_customer_uri(
            "postgresql://cloud_admin:secret@127.0.0.1:5432/postgres?sslmode=disable"
        ));
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
        assert!(
            statements
                .iter()
                .all(|statement| !statement.contains("cloud_admin"))
        );
        assert!(
            statements
                .iter()
                .any(|statement| statement.contains("VALID UNTIL '1970-01-01 00:15:00+00'"))
        );
        assert!(statements.iter().any(|statement| {
            statement.contains("GRANT CONNECT ON DATABASE postgres TO briven_a1b2c3d4e5f67890")
        }));
        let mut quoted = issued.clone();
        quoted.password = "aa'; drop role postgres; --".to_string();
        // pad check: a non-hex password must produce no SQL
        assert!(customer_role_statements(&quoted).is_none());
    }

    #[test]
    fn customer_uri_routes_to_one_registered_endpoint_without_option_injection() {
        let uri = customer_uri(
            "briven_a1b2c3d4e5f67890",
            PASSWORD,
            "db.internal",
            5432,
            "postgres",
        )
        .unwrap();
        let routed = super::with_endpoint_hint(&uri, "ep-a1b2c3d4e5f67890").unwrap();
        let url = url::Url::parse(&routed).unwrap();
        assert!(
            url.query_pairs()
                .any(|(key, value)| key == "options" && value == "endpoint=ep-a1b2c3d4e5f67890")
        );
        for id in [
            "../main",
            "ep-a --foo",
            "ep-a&role=cloud_admin",
            "other-tenant",
            "ep-A",
        ] {
            assert!(super::with_endpoint_hint(&uri, id).is_none());
        }
    }

    #[test]
    fn customer_connection_gate_refuses_viewers_before_a_missing_proxy() {
        use super::{ConnectionGate, connection_gate};
        assert_eq!(
            connection_gate(true, false, false, true),
            ConnectionGate::Forbidden
        );
    }

    #[test]
    fn customer_connection_gate_stays_unavailable_until_the_proxy_host_is_set() {
        use super::{ConnectionGate, connection_gate};
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
        use super::{ConnectionGate, connection_gate};
        assert_eq!(
            connection_gate(true, true, true, true),
            ConnectionGate::Issue
        );
        assert_eq!(
            connection_gate(false, false, false, false),
            ConnectionGate::Issue
        );
    }
}
