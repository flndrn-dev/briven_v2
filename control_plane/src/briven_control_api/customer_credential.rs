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
    password.len() == 64
        && password.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
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
