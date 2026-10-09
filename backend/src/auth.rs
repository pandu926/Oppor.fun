use crate::{
    crypto::random32,
    error::{ApiError, Result},
    http::ApiJson,
    state::AppState,
};
use alloy_primitives::{Address, keccak256};
use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{HeaderMap, header, request::Parts},
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use subtle::ConstantTimeEq;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Auth {
    pub user_id: Uuid,
    pub wallet: Address,
    pub session_id: Uuid,
}
pub struct MaybeAuth(pub Option<Auth>);
impl FromRequestParts<AppState> for MaybeAuth {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        if parts.headers.get_all(header::COOKIE).iter().any(|v| {
            v.to_str().is_ok_and(|s| {
                s.split(';').any(|p| {
                    p.trim()
                        .starts_with(&format!("{}=", state.config.cookie_name()))
                })
            })
        }) {
            Ok(Self(Some(Auth::from_request_parts(parts, state).await?)))
        } else {
            Ok(Self(None))
        }
    }
}
impl FromRequestParts<AppState> for Auth {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        let token = session_token(&parts.headers, state.config.cookie_name())?;
        let hash = keccak256(token.as_bytes());
        let row=sqlx::query("SELECT s.id,s.user_id,s.csrf_hash,s.contract_wallet,s.auth_message,s.auth_signature,u.primary_wallet,u.suspended FROM sessions s JOIN users u ON u.id=s.user_id WHERE token_hash=$1 AND s.chain_id=$2 AND revoked_at IS NULL AND expires_at>clock_timestamp()")
            .bind(hash.as_slice()).bind(state.config.chain_id as i64).fetch_optional(&state.db).await?.ok_or_else(ApiError::unauthorized)?;
        if row.try_get::<bool, _>("suspended")? {
            return Err(ApiError::forbidden());
        }
        if !matches!(
            parts.method,
            axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
        ) {
            let csrf = parts
                .headers
                .get("x-csrf-token")
                .and_then(|h| h.to_str().ok())
                .filter(|v| v.len() == 64)
                .ok_or_else(ApiError::forbidden)?;
            let expected: Vec<u8> = row.try_get("csrf_hash")?;
            if !bool::from(
                expected
                    .as_slice()
                    .ct_eq(keccak256(csrf.as_bytes()).as_slice()),
            ) {
                return Err(ApiError::forbidden());
            }
        }
        let wallet: Vec<u8> = row.try_get("primary_wallet")?;
        if row.try_get::<bool, _>("contract_wallet")?
            && !state
                .rpc
                .valid_wallet_signature(
                    Address::from_slice(&wallet),
                    &row.try_get::<String, _>("auth_message")?,
                    &row.try_get::<String, _>("auth_signature")?,
                )
                .await?
        {
            sqlx::query("UPDATE sessions SET revoked_at=clock_timestamp() WHERE id=$1")
                .bind(row.try_get::<Uuid, _>("id")?)
                .execute(&state.db)
                .await?;
            return Err(ApiError::unauthorized());
        }
        Ok(Self {
            user_id: row.try_get("user_id")?,
            wallet: Address::from_slice(&wallet),
            session_id: row.try_get("id")?,
        })
    }
}
fn session_token(headers: &HeaderMap, name: &str) -> Result<String> {
    let mut found = None;
    for value in headers.get_all(header::COOKIE).iter() {
        let raw = value.to_str().map_err(|_| ApiError::unauthorized())?;
        for part in raw.split(';') {
            if let Some((key, value)) = part.trim().split_once('=')
                && key == name
            {
                if found.is_some()
                    || value.len() != 64
                    || !value.bytes().all(|c| c.is_ascii_hexdigit())
                {
                    return Err(ApiError::unauthorized());
                }
                found = Some(value.to_owned());
            }
        }
    }
    found.ok_or_else(ApiError::unauthorized)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChallengeInput {
    pub wallet: Address,
    pub chain_id: String,
}
pub async fn challenge(
    State(state): State<AppState>,
    ApiJson(input): ApiJson<ChallengeInput>,
) -> Result<Json<serde_json::Value>> {
    if input.wallet.is_zero() || input.chain_id != state.config.chain_id.to_string() {
        return Err(ApiError::invalid("Invalid wallet or unsupported chain."));
    }
    let nonce = hex::encode(random32());
    let now = Utc::now();
    let expires = now + Duration::minutes(5);
    let message = format!(
        "{} wants you to sign in with your Ethereum account:\n{}\n\nSign in to Oppor. This request does not authorize any blockchain transaction.\n\nURI: {}\nVersion: 1\nChain ID: {}\nNonce: {}\nIssued At: {}\nExpiration Time: {}",
        state.config.domain,
        input.wallet.to_checksum(None),
        state.config.public_origin,
        state.config.chain_id,
        nonce,
        now.to_rfc3339(),
        expires.to_rfc3339()
    );
    // The parser validates EIP-4361 grammar; the exact server-issued bytes are bound in storage.
    let _: siwe::Message = message.parse().map_err(|_| ApiError::internal())?;
    sqlx::query("INSERT INTO auth_nonces(nonce_hash,wallet,chain_id,message_hash,expires_at) VALUES($1,$2,$3,$4,$5)")
        .bind(keccak256(nonce.as_bytes()).as_slice()).bind(input.wallet.as_slice()).bind(state.config.chain_id as i64).bind(keccak256(message.as_bytes()).as_slice()).bind(expires).execute(&state.db).await?;
    Ok(Json(json!({"message":message,"expires_at":expires})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyInput {
    pub message: String,
    pub signature: String,
}
pub async fn verify(
    State(state): State<AppState>,
    ApiJson(input): ApiJson<VerifyInput>,
) -> Result<Response> {
    if input.message.len() > 4096 || input.signature.len() > 8194 {
        return Err(ApiError::bad(
            "INVALID_MESSAGE",
            "The sign-in payload is too large.",
        ));
    }
    let parsed: siwe::Message = input
        .message
        .parse()
        .map_err(|_| ApiError::bad("INVALID_MESSAGE", "Invalid sign-in message."))?;
    if parsed.domain.as_str() != state.config.domain
        || parsed.uri.as_str() != state.config.public_origin
        || parsed.chain_id != state.config.chain_id
        || parsed.nonce.len() != 64
    {
        return Err(ApiError::unauthorized());
    }
    let nonce_hash = keccak256(parsed.nonce.as_bytes());
    let row=sqlx::query("SELECT wallet,message_hash FROM auth_nonces WHERE nonce_hash=$1 AND consumed_at IS NULL AND expires_at>clock_timestamp()")
        .bind(nonce_hash.as_slice()).fetch_optional(&state.db).await?.ok_or_else(ApiError::unauthorized)?;
    let wallet: Vec<u8> = row.try_get("wallet")?;
    let message_hash: Vec<u8> = row.try_get("message_hash")?;
    if wallet != parsed.address || message_hash != keccak256(input.message.as_bytes()).as_slice() {
        return Err(ApiError::unauthorized());
    }
    if !state
        .rpc
        .valid_wallet_signature(
            Address::from(parsed.address),
            &input.message,
            &input.signature,
        )
        .await?
    {
        return Err(ApiError::unauthorized());
    }
    let contract_wallet = !state
        .rpc
        .code(Address::from(parsed.address))
        .await?
        .is_empty();
    let mut tx = state.db.begin().await?;
    let consumed=sqlx::query("UPDATE auth_nonces SET consumed_at=clock_timestamp() WHERE nonce_hash=$1 AND consumed_at IS NULL AND expires_at>clock_timestamp()")
        .bind(nonce_hash.as_slice()).execute(&mut *tx).await?;
    if consumed.rows_affected() != 1 {
        return Err(ApiError::unauthorized());
    }
    let user_id:Uuid=sqlx::query_scalar("INSERT INTO users(id,primary_wallet) VALUES($1,$2) ON CONFLICT(primary_wallet) DO UPDATE SET primary_wallet=EXCLUDED.primary_wallet RETURNING id")
        .bind(Uuid::new_v4()).bind(&wallet).fetch_one(&mut *tx).await?;
    let suspended: bool = sqlx::query_scalar("SELECT suspended FROM users WHERE id=$1")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
    if suspended {
        return Err(ApiError::forbidden());
    }
    let token = hex::encode(random32());
    let csrf = hex::encode(random32());
    let session = Uuid::new_v4();
    sqlx::query("INSERT INTO sessions(id,user_id,token_hash,csrf_hash,chain_id,expires_at,contract_wallet,auth_message,auth_signature) VALUES($1,$2,$3,$4,$5,clock_timestamp()+make_interval(secs=>$6),$7,$8,$9)")
        .bind(session).bind(user_id).bind(keccak256(token.as_bytes()).as_slice()).bind(keccak256(csrf.as_bytes()).as_slice()).bind(state.config.chain_id as i64).bind(state.config.session_ttl as f64)
        .bind(contract_wallet).bind(if contract_wallet{Some(&input.message)}else{None}).bind(if contract_wallet{Some(&input.signature)}else{None}).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO audit_logs(id,actor_id,action) VALUES($1,$2,'SESSION_CREATED')")
        .bind(Uuid::new_v4())
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        state.config.cookie_name(),
        token,
        state.config.session_ttl,
        if state.config.production {
            "; Secure"
        } else {
            ""
        }
    );
    Ok(([(header::SET_COOKIE,cookie)],Json(json!({"user_id":user_id,"wallet":Address::from_slice(&wallet),"csrf_token":csrf,"expires_in":state.config.session_ttl}))).into_response())
}
pub async fn logout(State(state): State<AppState>, auth: Auth) -> Result<Response> {
    sqlx::query("UPDATE sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(auth.session_id)
        .execute(&state.db)
        .await?;
    let cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        state.config.cookie_name(),
        if state.config.production {
            "; Secure"
        } else {
            ""
        }
    );
    Ok((
        [(header::SET_COOKIE, cookie)],
        Json(json!({"logged_out":true})),
    )
        .into_response())
}
pub async fn me(auth: Auth) -> Json<serde_json::Value> {
    Json(json!({"user_id":auth.user_id,"wallet":auth.wallet}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_duplicate_session_cookie() {
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            format!(
                "oppor_session={}; oppor_session={}",
                "a".repeat(64),
                "b".repeat(64)
            )
            .parse()
            .unwrap(),
        );
        assert!(session_token(&h, "oppor_session").is_err());
    }
}
