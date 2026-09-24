use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub struct AuthConfig {
    keys: Vec<(String, [u8; 32])>,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self> {
        match (
            std::env::var("BRIVEN_CONTROL_API_KEYS_JSON").ok(),
            std::env::var("BRIVEN_CONTROL_API_TOKEN").ok(),
        ) {
            (Some(_), Some(_)) => bail!("set only one Briven control API credential variable"),
            (Some(json), None) => Self::from_json(&json),
            (None, Some(token)) => Self::from_keys(HashMap::from([("local".to_string(), token)])),
            (None, None) => {
                bail!("BRIVEN_CONTROL_API_KEYS_JSON or BRIVEN_CONTROL_API_TOKEN is required")
            }
        }
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
        Ok(Self { keys: result })
    }

    pub fn organization_for(&self, token: &str) -> Option<&str> {
        let hash: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        self.keys
            .iter()
            .find(|(_, expected)| bool::from(expected.ct_eq(&hash)))
            .map(|(organization, _)| organization.as_str())
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

#[cfg(test)]
mod tests {
    use super::AuthConfig;

    #[test]
    fn keys_identify_one_organization() {
        let auth = AuthConfig::from_json(
            r#"{"alpha":"alpha-token-012345678901234567890123", "beta":"beta-token-0123456789012345678901234"}"#,
        )
        .unwrap();
        assert_eq!(
            auth.organization_for("alpha-token-012345678901234567890123"),
            Some("alpha")
        );
        assert_eq!(
            auth.organization_for("beta-token-0123456789012345678901234"),
            Some("beta")
        );
        assert_eq!(
            auth.organization_for("wrong-token-012345678901234567890123"),
            None
        );
    }

    #[test]
    fn shared_keys_are_rejected() {
        assert!(AuthConfig::from_json(
            r#"{"alpha":"same-token-0123456789012345678901234", "beta":"same-token-0123456789012345678901234"}"#
        ).is_err());
    }
}
