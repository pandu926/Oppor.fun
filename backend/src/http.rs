use crate::{
    auth,
    error::{ApiError, Result},
    services,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{ConnectInfo, FromRequest, Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::{net::SocketAddr, sync::atomic::Ordering, time::Instant};
use tower_http::{catch_panic::CatchPanicLayer, cors::CorsLayer};
use uuid::Uuid;

pub struct ApiJson<T>(pub T);
impl<T: DeserializeOwned + Send> FromRequest<AppState> for ApiJson<T> {
    type Rejection = ApiError;
    async fn from_request(req: Request, state: &AppState) -> Result<Self> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|e| {
            if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
                ApiError {
                    status: e.status(),
                    code: "PAYLOAD_TOO_LARGE",
                    message: "The request exceeds the allowed body size.",
                }
            } else {
                ApiError::bad(
                    "INVALID_JSON",
                    "The JSON payload is invalid or contains unsupported fields.",
                )
            }
        })?;
        Ok(Self(value))
    }
}

pub fn router(state: AppState) -> Router {
    let origins = state
        .config
        .origins
        .iter()
        .map(|v| v.parse::<HeaderValue>().expect("Validated origins"))
        .collect::<Vec<_>>();
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-csrf-token"),
            header::HeaderName::from_static("idempotency-key"),
        ])
        .expose_headers([header::HeaderName::from_static("x-request-id")])
        .max_age(std::time::Duration::from_secs(600));
    let routes = Router::new()
        .route("/admin/me", get(services::admin::me))
        .route("/admin/stats", get(services::admin::stats))
        .route("/admin/campaigns", get(services::admin::campaigns))
        .route(
            "/admin/campaigns/{id}/moderation",
            post(services::admin::moderate),
        )
        .route("/admin/users", get(services::admin::users))
        .route(
            "/admin/users/{id}/suspension",
            post(services::admin::suspend),
        )
        .route(
            "/admin/users/{id}/revoke-sessions",
            post(services::admin::revoke_sessions),
        )
        .route("/admin/audit-logs", get(services::admin::audit_logs))
        .route("/admin/jobs", get(services::admin::jobs))
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .route("/metrics", get(metrics))
        .route("/auth/challenge", post(auth::challenge))
        .route("/auth/verify", post(auth::verify))
        .route("/auth/logout", post(auth::logout))
        .route("/me", get(auth::me))
        .route(
            "/campaigns",
            get(services::campaigns::list).post(services::campaigns::create),
        )
        .route("/campaigns/mine", get(services::campaigns::mine))
        .route(
            "/campaigns/{id}",
            get(services::campaigns::detail).patch(services::campaigns::update),
        )
        .route("/campaigns/{id}/tasks", post(services::campaigns::add_task))
        .route(
            "/campaigns/{id}/tasks/{task_id}",
            axum::routing::patch(services::campaigns::update_task)
                .delete(services::campaigns::delete_task),
        )
        .route(
            "/campaigns/{id}/lock-config",
            post(services::campaigns::lock_config),
        )
        .route(
            "/campaigns/{id}/entries",
            get(services::entries::list).post(services::entries::register),
        )
        .route("/campaigns/{id}/my-entry", get(services::entries::mine))
        .route(
            "/campaigns/{id}/my-entry/submissions/{task_id}",
            put(services::entries::evidence),
        )
        .route(
            "/campaigns/{id}/my-entry/submit",
            post(services::entries::submit),
        )
        .route(
            "/campaigns/{id}/entries/{entry_id}/evidence",
            get(services::entries::read_evidence),
        )
        .route(
            "/campaigns/{id}/entries/{entry_id}/review",
            post(services::entries::review),
        )
        .route(
            "/campaigns/{id}/lock-eligibility",
            post(services::allocations::lock),
        )
        .route(
            "/campaigns/{id}/allocation-preview",
            get(services::allocations::preview),
        )
        .route(
            "/campaigns/{id}/results",
            get(services::allocations::results),
        )
        .route(
            "/campaigns/{id}/manifest",
            get(services::allocations::manifest),
        )
        .route(
            "/campaigns/{id}/allocations/{wallet}",
            get(services::allocations::proofs),
        )
        .route(
            "/campaigns/{id}/prepare-create",
            post(services::transactions::create),
        )
        .route(
            "/campaigns/{id}/prepare-fund",
            post(services::transactions::fund),
        )
        .route(
            "/campaigns/{id}/prepare-activate",
            post(services::transactions::activate),
        )
        .route(
            "/campaigns/{id}/prepare-finalize",
            post(services::transactions::finalize),
        )
        .route(
            "/campaigns/{id}/prepare-claim",
            post(services::transactions::claim),
        )
        .route(
            "/campaigns/{id}/prepare-cancel",
            post(services::transactions::cancel),
        )
        .route(
            "/campaigns/{id}/prepare-sweep",
            post(services::transactions::sweep),
        )
        .route(
            "/campaigns/{id}/transactions",
            post(services::transactions::track),
        )
        .route("/uploads/presign", post(services::uploads::presign))
        .route(
            "/uploads/{upload_id}/complete",
            post(services::uploads::complete),
        )
        .fallback(|| async { ApiError::missing() })
        .layer(axum::extract::DefaultBodyLimit::max(64 * 1024));
    Router::new()
        .nest("/v1", routes)
        .layer(cors)
        .layer(CatchPanicLayer::custom(|_| {
            ApiError::internal().into_response()
        }))
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}

async fn guard(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    state.metrics.requests.fetch_add(1, Ordering::Relaxed);
    req.extensions_mut().insert(request_id.clone());
    let outcome=async {
        let _permit=state.concurrent.clone().try_acquire_owned().map_err(|_|ApiError::unavailable())?;
        if !matches!(method,Method::GET|Method::HEAD|Method::OPTIONS){
            let origin=req.headers().get(header::ORIGIN).and_then(|v|v.to_str().ok()).ok_or_else(ApiError::forbidden)?;
            if !state.config.origins.iter().any(|v|v==origin){return Err(ApiError::forbidden());}
        }
        // Forwarded addresses are accepted only from explicitly trusted network peers.
        if !matches!(path.as_str(),"/v1/health/live"|"/v1/health/ready"|"/v1/metrics") && method!=Method::OPTIONS {
            let peer=req.extensions().get::<ConnectInfo<SocketAddr>>().map(|v|v.0.ip());
            let ip=match peer{Some(peer)=>client_ip(req.headers(),peer,&state.config.trusted_proxies)?.to_string(),None=>"unknown-peer".into()};
            let bucket=if path.starts_with("/v1/auth/"){"auth"}else if method==Method::GET{"read"}else{"write"};
            let limit=match bucket{"auth"=>20,"read"=>300,_=>120};
            let mut material=state.config.rate_key.to_vec();material.extend(ip.as_bytes());
            let key=format!("oppor:rl:{}:{bucket}",alloy_primitives::keccak256(material));
            let script=redis::Script::new("local n=redis.call('INCR',KEYS[1]); if n==1 then redis.call('EXPIRE',KEYS[1],60) end; return n");
            let count:std::result::Result<i64,_>=tokio::time::timeout(std::time::Duration::from_millis(500),script.key(key).invoke_async(&mut state.redis.clone())).await.unwrap_or_else(|_|Err(redis::RedisError::from((redis::ErrorKind::IoError,"Redis timeout"))));
            match count{Ok(n) if n>limit=>return Err(ApiError{status:StatusCode::TOO_MANY_REQUESTS,code:"RATE_LIMITED",message:"Too many requests. Retry after the current one-minute window."}),Err(_) if method!=Method::GET=>return Err(ApiError::unavailable()),_=>{}}
        }
        tokio::time::timeout(state.config.http_timeout,next.run(req)).await.map_err(|_|ApiError{status:StatusCode::GATEWAY_TIMEOUT,code:"REQUEST_TIMEOUT",message:"The request timed out. Retry mutations with the same idempotency key."})
    }.await;
    let mut response = match outcome {
        Ok(r) => r,
        Err(e) => e.into_response(),
    };
    if response.status().is_client_error() || response.status().is_server_error() {
        let status = response.status();
        let (headers, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, 64 * 1024)
            .await
            .unwrap_or_default();
        let mut value:serde_json::Value=serde_json::from_slice(&bytes).unwrap_or_else(|_|json!({"error":{"code":"HTTP_ERROR","message":"The request could not be completed."}}));
        if value.get("error").is_none() {
            value = json!({"error":{"code":"HTTP_ERROR","message":"The request could not be completed."}});
        }
        value["error"]["request_id"] = json!(&request_id);
        response = (status, Json(value)).into_response();
        // Preserve protocol headers such as Allow and CORS headers from rejection responses.
        for (name, value) in &headers.headers {
            if name != header::CONTENT_LENGTH && name != header::CONTENT_TYPE {
                response.headers_mut().insert(name.clone(), value.clone());
            }
        }
    }
    if response.status().is_client_error() || response.status().is_server_error() {
        state.metrics.failures.fetch_add(1, Ordering::Relaxed);
    }
    let elapsed = start.elapsed().as_micros() as u64;
    state
        .metrics
        .latency_micros
        .fetch_add(elapsed, Ordering::Relaxed);
    response
        .headers_mut()
        .insert("x-request-id", request_id.parse().expect("UUID header"));
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response.headers_mut().insert(
        "content-security-policy",
        HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
    );
    if state.config.production {
        response.headers_mut().insert(
            "strict-transport-security",
            HeaderValue::from_static("max-age=31536000"),
        );
    }
    tracing::info!(%request_id,%method,%path,status=response.status().as_u16(),duration_micros=elapsed,"HTTP request completed");
    response
}
async fn live() -> Json<serde_json::Value> {
    Json(json!({"status":"ok"}))
}
pub fn client_ip(
    headers: &axum::http::HeaderMap,
    peer: std::net::IpAddr,
    trusted: &[ipnet::IpNet],
) -> Result<std::net::IpAddr> {
    if !trusted.iter().any(|n| n.contains(&peer)) {
        return Ok(peer);
    }
    let Some(raw) = headers.get("x-forwarded-for") else {
        return Ok(peer);
    };
    let raw = raw.to_str().map_err(|_| {
        ApiError::bad(
            "INVALID_FORWARDED_HEADER",
            "Invalid forwarded address header.",
        )
    })?;
    if raw.len() > 1024 {
        return Err(ApiError::bad(
            "INVALID_FORWARDED_HEADER",
            "Invalid forwarded address header.",
        ));
    }
    let addresses = raw
        .split(',')
        .map(|v| v.trim().parse::<std::net::IpAddr>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| {
            ApiError::bad(
                "INVALID_FORWARDED_HEADER",
                "Invalid forwarded address header.",
            )
        })?;
    if addresses.len() > 10 || addresses.is_empty() {
        return Err(ApiError::bad(
            "INVALID_FORWARDED_HEADER",
            "Invalid forwarded address header.",
        ));
    }
    for ip in addresses.iter().rev() {
        if !trusted.iter().any(|n| n.contains(ip)) {
            return Ok(*ip);
        }
    }
    Ok(addresses[0])
}

async fn ready(State(state): State<AppState>) -> Result<Json<serde_json::Value>> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    let pong: std::result::Result<String, _> = redis::cmd("PING")
        .query_async(&mut state.redis.clone())
        .await;
    if pong.is_err() {
        return Err(ApiError::unavailable());
    }
    state.ensure_indexer().await?;
    Ok(Json(json!({"status":"ready"})))
}
async fn metrics(State(state): State<AppState>) -> Response {
    let m = &state.metrics;
    let indexed_at: i64 = sqlx::query_scalar("SELECT coalesce(max(extract(epoch FROM updated_at))::bigint,0) FROM indexer_cursors WHERE chain_id=$1 AND factory_address=$2 AND observed_safe_head IS NOT NULL AND halted_reason IS NULL")
        .bind(state.config.chain_id as i64).bind(state.config.factory.as_slice())
        .fetch_one(&state.db).await.unwrap_or(0);
    let body = format!(
        "# TYPE oppor_http_requests_total counter\noppor_http_requests_total {}\n# TYPE oppor_http_failures_total counter\noppor_http_failures_total {}\n# TYPE oppor_http_duration_seconds_sum counter\noppor_http_duration_seconds_sum {}\n# TYPE oppor_database_pool_connections gauge\noppor_database_pool_connections {}\n",
        m.requests.load(Ordering::Relaxed),
        m.failures.load(Ordering::Relaxed),
        m.latency_micros.load(Ordering::Relaxed) as f64 / 1_000_000.,
        state.db.size()
    );
    let body = format!(
        "{body}# TYPE oppor_database_pool_idle gauge\noppor_database_pool_idle {}\n# TYPE oppor_indexer_last_verified_timestamp_seconds gauge\noppor_indexer_last_verified_timestamp_seconds {}\n",
        state.db.num_idle(),
        indexed_at
    );
    ([(header::CONTENT_TYPE, "text/plain; version=0.0.4")], body).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forwarded_headers_require_trust() {
        let mut h = axum::http::HeaderMap::new();
        h.insert(
            "x-forwarded-for",
            "1.2.3.4, 203.0.113.4, 10.0.0.2".parse().unwrap(),
        );
        let peer = "10.0.0.1".parse().unwrap();
        assert_eq!(client_ip(&h, peer, &[]).unwrap(), peer);
        assert_eq!(
            client_ip(&h, peer, &["10.0.0.0/24".parse().unwrap()]).unwrap(),
            "203.0.113.4".parse::<std::net::IpAddr>().unwrap()
        );
    }
    #[test]
    fn malformed_forwarded_header_is_rejected() {
        let mut h = axum::http::HeaderMap::new();
        h.insert("x-forwarded-for", "unknown".parse().unwrap());
        assert!(
            client_ip(
                &h,
                "10.0.0.1".parse().unwrap(),
                &["10.0.0.0/24".parse().unwrap()]
            )
            .is_err()
        );
    }
}
