//! Adapter for the inherited Neon proxy's private control-plane protocol.
//! Password verifiers are deliberately neither Debug nor logged.
use super::*;
use axum::extract::Query;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub(super) struct ProxyAuth(Option<[u8; 32]>);

impl ProxyAuth {
    pub(super) fn from_env() -> Result<Self> {
        let secret = std::env::var("BRIVEN_PROXY_CONTROL_SECRET").ok();
        if secret.is_some() && secret == std::env::var("BRIVEN_CONTROL_IDENTITY_SECRET").ok() {
            bail!("proxy and website identity secrets must be independent");
        }
        Self::new(secret.as_deref())
    }

    fn new(secret: Option<&str>) -> Result<Self> {
        match secret {
            Some(secret) if secret.len() >= 32 => Ok(Self(Some(Sha256::digest(secret).into()))),
            Some(_) => bail!("BRIVEN_PROXY_CONTROL_SECRET must be at least 32 characters"),
            None => Ok(Self(None)),
        }
    }

    fn accepts(&self, supplied: Option<&str>) -> bool {
        match (&self.0, supplied) {
            (Some(expected), Some(supplied)) => {
                let actual: [u8; 32] = Sha256::digest(supplied).into();
                bool::from(expected.ct_eq(&actual))
            }
            _ => false,
        }
    }

    fn authorize(&self, headers: &HeaderMap) -> ApiResult<()> {
        let supplied = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));
        if self.accepts(supplied) {
            Ok(())
        } else {
            Err(ApiError(
                StatusCode::UNAUTHORIZED,
                "authentication required",
            ))
        }
    }
}

#[derive(Deserialize)]
pub(super) struct ProxyQuery {
    endpointish: String,
    role: Option<String>,
}

#[derive(Serialize)]
pub(super) struct AccessControl {
    role_secret: String,
}

pub(super) async fn endpoint(
    state: &AppState,
    id: &str,
) -> ApiResult<Arc<control_plane::endpoint::Endpoint>> {
    let plane = load_plane(state)?;
    let endpoint = plane
        .endpoints
        .get(id)
        .cloned()
        .ok_or(ApiError(StatusCode::NOT_FOUND, "endpoint not found"))?;
    if state
        .catalog
        .ready_project(&endpoint.tenant_id.to_string())
        .await
        .map_err(internal)?
        .is_none()
    {
        return Err(ApiError(StatusCode::NOT_FOUND, "endpoint not found"));
    }
    lifecycle_runtime::demand(state, endpoint).await
}

pub(super) async fn access_control(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ProxyQuery>,
) -> ApiResult<Json<AccessControl>> {
    state.proxy_auth.authorize(&headers)?;
    let role = query
        .role
        .as_deref()
        .filter(|role| customer_credential::customer_door_accepts(role))
        .ok_or(ApiError(StatusCode::FORBIDDEN, "customer role required"))?;
    let _guard = state.gate.lock().await;
    // Validate binding before starting another tenant's compute.
    let plane = load_plane(&state)?;
    let candidate = plane
        .endpoints
        .get(&query.endpointish)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "endpoint not found"))?;
    if !customer_credential::credential_opens_project(role, &candidate.tenant_id.to_string()) {
        return Err(ApiError(StatusCode::FORBIDDEN, "customer role required"));
    }
    let endpoint = endpoint(&state, &query.endpointish).await?;
    let admin_uri = endpoint.connstr("cloud_admin", "postgres");
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        let (client, connection) =
            tokio_postgres::connect(&format!("{admin_uri}?sslmode=disable"), NoTls).await?;
        let task = scopeguard::guard(
            tokio::spawn(async move {
                let _ = connection.await;
            }),
            |task| task.abort(),
        );
        let row = client
            .query_opt(
                "SELECT rolpassword FROM pg_authid WHERE rolname = $1 AND rolcanlogin
             AND NOT rolsuper AND NOT rolcreaterole AND NOT rolcreatedb AND NOT rolreplication
             AND rolvaliduntil IS NOT NULL AND rolvaliduntil > CURRENT_TIMESTAMP",
                &[&role],
            )
            .await;
        task.abort();
        row
    })
    .await
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "compute unavailable"))?
    .map_err(engine)?;
    let secret: Option<String> = result.and_then(|row| row.get(0));
    let secret = secret
        .filter(|secret| secret.starts_with("SCRAM-SHA-256$"))
        .ok_or(ApiError(
            StatusCode::FORBIDDEN,
            "customer credential expired or unavailable",
        ))?;
    Ok(Json(AccessControl {
        role_secret: secret,
    }))
}

pub(super) async fn wake_compute(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ProxyQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    state.proxy_auth.authorize(&headers)?;
    let _guard = state.gate.lock().await;
    let endpoint = endpoint(&state, &query.endpointish).await?;
    Ok(Json(serde_json::json!({
        "address": endpoint.pg_address.to_string(),
        "aux": {
            "endpoint_id": query.endpointish, "compute_id": query.endpointish,
            "project_id": endpoint.tenant_id.to_string(),
            "branch_id": endpoint.timeline_id.to_string(), "cold_start_info": "warm"
        }
    })))
}

#[cfg(test)]
mod tests {
    use super::ProxyAuth;
    #[test]
    fn proxy_requires_its_own_secret_and_fails_closed_when_unconfigured() {
        let secret = "proxy-secret-aaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let auth = ProxyAuth::new(Some(secret)).unwrap();
        assert!(auth.accepts(Some(secret)));
        assert!(!auth.accepts(None));
        assert!(!auth.accepts(Some("website-identity-bbbbbbbbbbbbbbbbbbbbbbbb")));
        assert!(!auth.accepts(Some("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJvd25lciJ9.signature")));
        assert!(!ProxyAuth::new(None).unwrap().accepts(Some(secret)));
        assert!(ProxyAuth::new(Some("short")).is_err());
    }
}
