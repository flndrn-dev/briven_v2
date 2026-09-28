use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub struct AuthConfig {
    keys: Vec<(String, [u8; 32])>,
    identity_key: Option<DecodingKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Viewer,
    Developer,
    Admin,
    Owner,
}

pub struct Principal {
    pub organization: String,
    pub role: Role,
}

impl Principal {
    pub fn can_write(&self) -> bool {
        self.role != Role::Viewer
    }
}

#[derive(Deserialize)]
struct IdentityClaims {
    sub: String,
    org_id: String,
    role: String,
    iat: u64,
    exp: u64,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self> {
        let secret = std::env::var("BRIVEN_CONTROL_IDENTITY_SECRET").ok();
        let json = std::env::var("BRIVEN_CONTROL_API_KEYS_JSON").ok();
        let token = std::env::var("BRIVEN_CONTROL_API_TOKEN").ok();
        if let Some(secret) = secret {
            if json.is_some() || token.is_some() {
                bail!("signed customer identity cannot be combined with static control keys");
            }
            if secret.len() < 32 {
                bail!("BRIVEN_CONTROL_IDENTITY_SECRET must be at least 32 characters");
            }
            return Ok(Self {
                keys: Vec::new(),
                identity_key: Some(DecodingKey::from_secret(secret.as_bytes())),
            });
        }
        match (json, token) {
            (Some(_), Some(_)) => bail!("set only one Briven control API credential variable"),
            (Some(json), None) => Self::from_json(&json),
            (None, Some(token)) => Self::from_keys(HashMap::from([("local".to_string(), token)])),
            (None, None) => {
                bail!("BRIVEN_CONTROL_IDENTITY_SECRET or a development control key is required")
            }
        }
    }

    pub fn uses_customer_identity(&self) -> bool {
        self.identity_key.is_some()
    }

    fn from_json(json: &str) -> Result<Self> {
        Self::from_keys(serde_json::from_str(json)?)
    }

    fn from_keys(keys: HashMap<String, String>) -> Result<Self> {
        if keys.is_empty() {
            bail!("at least one organization API key is required");
        }
        let mut seen = HashSet::new();
        let mut result = Vec::with_capacity(keys.len());
        for (organization, token) in keys {
            if !valid_organization(&organization) {
                bail!("invalid organization identifier: {organization}");
            }
            if token.len() < 32 {
                bail!("each Briven control API key must be at least 32 characters");
            }
            let hash: [u8; 32] = Sha256::digest(token.as_bytes()).into();
            if !seen.insert(hash) {
                bail!("organization API keys must be distinct");
            }
            result.push((organization, hash));
        }
        Ok(Self {
            keys: result,
            identity_key: None,
        })
    }

    pub fn principal_for(&self, token: &str) -> Option<Principal> {
        if let Some(key) = &self.identity_key {
            let mut validation = Validation::new(Algorithm::HS256);
            validation.set_issuer(&["briven-api"]);
            validation.set_audience(&["briven-control"]);
            validation.leeway = 0;
            validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub"]);
            let claims = decode::<IdentityClaims>(token, key, &validation)
                .ok()?
                .claims;
            let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
            if claims.sub.is_empty()
                || claims.iat > now
                || claims.exp <= claims.iat
                || claims.exp - claims.iat > 60
                || !valid_customer_org(&claims.org_id)
            {
                return None;
            }
            let role = match claims.role.as_str() {
                "viewer" => Role::Viewer,
                "developer" => Role::Developer,
                "admin" => Role::Admin,
                "owner" => Role::Owner,
                _ => return None,
            };
            return Some(Principal {
                organization: claims.org_id,
                role,
            });
        }
        let hash: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        self.keys
            .iter()
            .find(|(_, expected)| bool::from(expected.ct_eq(&hash)))
            .map(|(organization, _)| Principal {
                organization: organization.clone(),
                role: Role::Admin,
            })
    }
}

fn valid_organization(name: &str) -> bool {
    let bytes = name.as_bytes();
    (1..=48).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        && bytes[bytes.len() - 1] != b'-'
}

fn valid_customer_org(name: &str) -> bool {
    let bytes = name.as_bytes();
    (1..=128).contains(&bytes.len())
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::{AuthConfig, Role};
    use jsonwebtoken::{DecodingKey, EncodingKey, Header, encode};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Claims<'a> {
        iss: &'a str,
        aud: &'a str,
        sub: &'a str,
        org_id: &'a str,
        role: &'a str,
        iat: u64,
        exp: u64,
    }

    #[test]
    fn static_keys_are_isolated() {
        let auth = AuthConfig::from_json(r#"{"alpha":"alpha-token-012345678901234567890123", "beta":"beta-token-0123456789012345678901234"}"#).unwrap();
        assert_eq!(
            auth.principal_for("alpha-token-012345678901234567890123")
                .unwrap()
                .organization,
            "alpha"
        );
        assert_eq!(
            auth.principal_for("beta-token-0123456789012345678901234")
                .unwrap()
                .organization,
            "beta"
        );
        assert!(
            auth.principal_for("wrong-token-012345678901234567890123")
                .is_none()
        );
        assert!(AuthConfig::from_json(r#"{"alpha":"same-token-0123456789012345678901234", "beta":"same-token-0123456789012345678901234"}"#).is_err());
    }

    #[test]
    fn signed_identity_checks_audience_role_and_lifetime() {
        let secret = "customer-identity-secret-at-least-32-characters";
        let auth = AuthConfig {
            keys: Vec::new(),
            identity_key: Some(DecodingKey::from_secret(secret.as_bytes())),
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let sign = |aud, role, exp| {
            encode(
                &Header::default(),
                &Claims {
                    iss: "briven-api",
                    aud,
                    sub: "user_1",
                    org_id: "org_user_1",
                    role,
                    iat: now,
                    exp,
                },
                &EncodingKey::from_secret(secret.as_bytes()),
            )
            .unwrap()
        };
        let principal = auth
            .principal_for(&sign("briven-control", "viewer", now + 60))
            .unwrap();
        assert_eq!(principal.organization, "org_user_1");
        assert_eq!(principal.role, Role::Viewer);
        assert!(!principal.can_write());
        assert!(
            auth.principal_for(&sign("wrong", "viewer", now + 60))
                .is_none()
        );
        assert!(
            auth.principal_for(&sign("briven-control", "owner", now + 61))
                .is_none()
        );
        assert!(
            auth.principal_for(&sign("briven-control", "owner", now - 1))
                .is_none()
        );
        assert!(
            auth.principal_for(&sign("briven-control", "unknown", now + 60))
                .is_none()
        );
    }
}
