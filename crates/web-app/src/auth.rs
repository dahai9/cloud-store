use crate::AppState;
use argon2::password_hash::{PasswordHash, PasswordHasher, SaltString};
use argon2::{Argon2, PasswordVerifier};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tracing::error;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Serialize)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub role: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthTokenResponse {
    pub token: String,
    pub user_id: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct AuthProfileResponse {
    pub user_id: String,
    pub email: String,
    pub role: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    email: String,
    role: String,
    exp: i64,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthTokenResponse>, (StatusCode, &'static str)> {
    if payload.email.trim().is_empty() || payload.password.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "email and password are required"));
    }

    let user_id = Uuid::new_v4().to_string();
    let password_hash = hash_password(&payload.password)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to hash password"))?;

    sqlx::query("INSERT INTO users (id, email, password_hash, role) VALUES (?, ?, ?, 'user')")
        .bind(&user_id)
        .bind(payload.email.trim().to_lowercase())
        .bind(&password_hash)
        .execute(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, "failed to create user");
            if err.to_string().contains("UNIQUE constraint failed") {
                (StatusCode::CONFLICT, "email already exists")
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "failed to create user")
            }
        })?;

    let token = issue_token(
        &state.session_secret,
        &user_id,
        payload.email.trim(),
        "user",
        60 * 60 * 24,
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to issue token"))?;

    Ok(Json(AuthTokenResponse {
        token,
        user_id,
        role: "user".to_string(),
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthTokenResponse>, (StatusCode, &'static str)> {
    let email = payload.email.trim().to_lowercase();

    let user = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, password_hash, role FROM users WHERE email = ? LIMIT 1",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to query user by email");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to login")
    })?
    .ok_or((StatusCode::UNAUTHORIZED, "invalid credentials"))?;

    let (user_id, password_hash, role) = user;
    let is_valid = verify_password(&payload.password, &password_hash);

    if !is_valid {
        return Err((StatusCode::UNAUTHORIZED, "invalid credentials"));
    }

    let token = issue_token(&state.session_secret, &user_id, &email, &role, 60 * 60 * 24)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to issue token"))?;

    Ok(Json(AuthTokenResponse {
        token,
        user_id,
        role,
    }))
}

pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthProfileResponse>, (StatusCode, &'static str)> {
    let user = require_auth(&headers, &state)?;

    Ok(Json(AuthProfileResponse {
        user_id: user.id,
        email: user.email,
        role: user.role,
    }))
}

pub fn require_auth(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<AuthUser, (StatusCode, &'static str)> {
    let token = bearer_token(headers).ok_or((StatusCode::UNAUTHORIZED, "missing bearer token"))?;

    let claims = parse_token(token, &state.session_secret)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid token"))?;

    if claims.exp <= Utc::now().timestamp() {
        return Err((StatusCode::UNAUTHORIZED, "token expired"));
    }

    Ok(AuthUser {
        id: claims.sub,
        email: claims.email,
        role: claims.role,
    })
}

pub fn require_admin(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<AuthUser, (StatusCode, &'static str)> {
    let user = require_auth(headers, state)?;

    if user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "admin role required"));
    }

    Ok(user)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(axum::http::header::AUTHORIZATION)?;
    let text = value.to_str().ok()?;
    text.strip_prefix("Bearer ")
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes())
        .expect("uuid bytes should always encode to valid salt");
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

fn verify_password(password: &str, stored: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored) else {
        return false;
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn issue_token(
    session_secret: &str,
    user_id: &str,
    email: &str,
    role: &str,
    ttl_seconds: i64,
) -> Result<String, serde_json::Error> {
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        exp: Utc::now().timestamp() + ttl_seconds,
    };

    let claims_json = serde_json::to_vec(&claims)?;
    let payload = URL_SAFE_NO_PAD.encode(claims_json);
    let signature = sign_payload(session_secret, payload.as_bytes());

    Ok(format!("{payload}.{signature}"))
}

fn parse_token(token: &str, session_secret: &str) -> Result<Claims, &'static str> {
    let mut split = token.split('.');
    let payload = split.next().ok_or("malformed token")?;
    let signature = split.next().ok_or("malformed token")?;

    if split.next().is_some() {
        return Err("malformed token");
    }

    let expected = sign_payload(session_secret, payload.as_bytes());
    if signature != expected {
        return Err("signature mismatch");
    }

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| "invalid payload")?;

    serde_json::from_slice::<Claims>(&payload_bytes).map_err(|_| "invalid payload")
}

fn sign_payload(session_secret: &str, payload: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(session_secret.as_bytes()).expect("hmac accepts any key length");
    mac.update(payload);
    let bytes = mac.finalize().into_bytes();
    URL_SAFE_NO_PAD.encode(bytes)
}
