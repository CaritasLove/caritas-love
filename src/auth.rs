// auth.rs
// Copyright 2026 Patrick Meade.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use log::{info, warn};
use rand::prelude::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

use crate::{AppState, app_env::AppEnv};

pub const ADMIN_SESSION_COOKIE: &str = "admin_session";
pub const MIN_PASSWORD_LENGTH: usize = 12;
const SESSION_TOKEN_BYTES: usize = 32;
const SESSION_IDLE_TIMEOUT_SECONDS: i64 = 2 * 60 * 60;
const SESSION_ABSOLUTE_TIMEOUT_SECONDS: i64 = 8 * 60 * 60;

#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub bootstrap_admin_username: Option<String>,
    pub bootstrap_admin_password: Option<String>,
    pub cookie_secure: bool,
}

impl AuthConfig {
    pub fn from_system(app_env: AppEnv) -> Self {
        let cookie_secure = match std::env::var("AUTH_COOKIE_SECURE") {
            Ok(raw) => parse_bool_env(&raw).unwrap_or(app_env == AppEnv::Production),
            Err(_) => app_env == AppEnv::Production,
        };

        Self {
            bootstrap_admin_username: optional_env("BOOTSTRAP_ADMIN_USERNAME"),
            bootstrap_admin_password: optional_env("BOOTSTRAP_ADMIN_PASSWORD"),
            cookie_secure,
        }
    }

    pub fn session_cookie(&self, token: String) -> Cookie<'static> {
        Cookie::build((ADMIN_SESSION_COOKIE, token))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(self.cookie_secure)
            .build()
    }

    pub fn clear_session_cookie(&self) -> Cookie<'static> {
        let mut cookie = Cookie::build((ADMIN_SESSION_COOKIE, ""))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(self.cookie_secure)
            .build();
        cookie.make_removal();
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_same_site(SameSite::Lax);
        cookie.set_secure(self.cookie_secure);
        cookie
    }
}

#[derive(Clone, Debug)]
pub struct AdminUser {
    pub id: i64,
    pub username: String,
    pub force_password_change: bool,
}

#[derive(Clone, Debug)]
pub struct CurrentAdmin {
    pub user: AdminUser,
    pub session_token_hash: String,
}

#[derive(Debug)]
pub enum AuthError {
    InvalidCredentials,
    MissingBootstrapCredentials,
    PasswordTooShort,
    PasswordHash,
    Query(sqlx::Error),
}

impl From<sqlx::Error> for AuthError {
    fn from(error: sqlx::Error) -> Self {
        Self::Query(error)
    }
}

impl<S> FromRequestParts<S> for CurrentAdmin
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = ();

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let jar = CookieJar::from_headers(&parts.headers);
        let Some(cookie) = jar.get(ADMIN_SESSION_COOKIE) else {
            return Err(());
        };

        let Some(current_admin) =
            load_current_admin(&app_state.db, cookie.value(), SESSION_IDLE_TIMEOUT_SECONDS)
                .await
                .map_err(|_| ())?
        else {
            return Err(());
        };

        Ok(current_admin)
    }
}

#[derive(Deserialize)]
pub struct AdminLoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct PasswordChangeForm {
    pub mode: String,
    pub target_admin_id: i64,
    pub current_password: Option<String>,
    pub temporary_password: Option<String>,
    pub new_password: String,
    pub confirm_password: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PasswordResetMode {
    Change,
    AdminReset,
}

impl PasswordResetMode {
    pub fn from_query_value(value: Option<&str>, force_password_change: bool) -> Self {
        if force_password_change {
            return Self::Change;
        }

        match value {
            Some("change") => Self::Change,
            _ => Self::AdminReset,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Change => "change",
            Self::AdminReset => "admin-reset",
        }
    }
}

pub fn verify_password(password_hash: &str, candidate: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
        return false;
    };

    password_hasher()
        .verify_password(candidate.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(AuthError::PasswordTooShort);
    }

    let salt = SaltString::generate(&mut OsRng);

    password_hasher()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthError::PasswordHash)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn bootstrap_admin_if_needed(
    pool: &PgPool,
    auth_config: &AuthConfig,
) -> Result<(), AuthError> {
    let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_user")
        .fetch_one(pool)
        .await?;

    if admin_count > 0 {
        return Ok(());
    }

    let Some(username) = auth_config.bootstrap_admin_username.as_deref() else {
        warn!("no admin users exist and BOOTSTRAP_ADMIN_USERNAME is not set");
        return Ok(());
    };

    let Some(password) = auth_config.bootstrap_admin_password.as_deref() else {
        warn!("no admin users exist and BOOTSTRAP_ADMIN_PASSWORD is not set");
        return Err(AuthError::MissingBootstrapCredentials);
    };

    let password_hash = hash_password(password)?;

    sqlx::query(
        r#"
        INSERT INTO admin_user (
            username,
            password_hash,
            force_password_change,
            active
        )
        SELECT $1, $2, TRUE, TRUE
        WHERE NOT EXISTS (SELECT 1 FROM admin_user)
        "#,
    )
    .bind(username)
    .bind(password_hash)
    .execute(pool)
    .await?;

    info!("bootstrapped initial admin account '{}'", username);
    Ok(())
}

pub async fn authenticate_admin(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<Option<AdminUser>, AuthError> {
    let row = sqlx::query(
        r#"
        SELECT id, username, password_hash, force_password_change, active
        FROM admin_user
        WHERE username = $1
        "#,
    )
    .bind(username.trim())
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let active: bool = row.try_get("active")?;
    if !active {
        return Ok(None);
    }

    let password_hash: String = row.try_get("password_hash")?;
    if !verify_password(&password_hash, password) {
        return Ok(None);
    }

    Ok(Some(AdminUser {
        id: row.try_get("id")?,
        username: row.try_get("username")?,
        force_password_change: row.try_get("force_password_change")?,
    }))
}

pub async fn create_admin_session(pool: &PgPool, admin_user_id: i64) -> Result<String, AuthError> {
    let token = random_hex(SESSION_TOKEN_BYTES);
    let token_hash = hash_token(&token);

    sqlx::query(
        r#"
        INSERT INTO admin_session (
            admin_user_id,
            session_token_hash,
            created_at,
            last_seen_at,
            expires_at
        )
        VALUES ($1, $2, NOW(), NOW(), NOW() + make_interval(secs => $3))
        "#,
    )
    .bind(admin_user_id)
    .bind(&token_hash)
    .bind(SESSION_ABSOLUTE_TIMEOUT_SECONDS)
    .execute(pool)
    .await?;

    Ok(token)
}

pub async fn revoke_session_by_token(pool: &PgPool, token: &str) -> Result<(), AuthError> {
    let token_hash = hash_token(token);

    sqlx::query(
        r#"
        UPDATE admin_session
        SET revoked_at = NOW()
        WHERE session_token_hash = $1
          AND revoked_at IS NULL
        "#,
    )
    .bind(token_hash)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn change_own_password(
    pool: &PgPool,
    admin_user: &AdminUser,
    keep_session_token_hash: &str,
    current_password: &str,
    new_password: &str,
) -> Result<(), AuthError> {
    let row = sqlx::query("SELECT password_hash FROM admin_user WHERE id = $1 AND active = TRUE")
        .bind(admin_user.id)
        .fetch_optional(pool)
        .await?;

    let Some(row) = row else {
        return Err(AuthError::InvalidCredentials);
    };

    let password_hash: String = row.try_get("password_hash")?;
    if !verify_password(&password_hash, current_password) {
        return Err(AuthError::InvalidCredentials);
    }

    let new_password_hash = hash_password(new_password)?;

    sqlx::query(
        r#"
        UPDATE admin_user
        SET password_hash = $2,
            force_password_change = FALSE,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(admin_user.id)
    .bind(new_password_hash)
    .execute(pool)
    .await?;

    revoke_all_other_sessions(pool, admin_user.id, Some(keep_session_token_hash)).await?;

    Ok(())
}

pub async fn admin_reset_password(
    pool: &PgPool,
    actor_admin_id: i64,
    target_admin_id: i64,
    temporary_password: &str,
) -> Result<(), AuthError> {
    let temporary_password_hash = hash_password(temporary_password)?;

    sqlx::query(
        r#"
        UPDATE admin_user
        SET password_hash = $2,
            force_password_change = TRUE,
            updated_at = NOW()
        WHERE id = $1
          AND active = TRUE
        "#,
    )
    .bind(target_admin_id)
    .bind(temporary_password_hash)
    .execute(pool)
    .await?;

    revoke_all_other_sessions(pool, target_admin_id, None).await?;

    record_audit_event(
        pool,
        Some(actor_admin_id),
        Some(target_admin_id),
        "admin_password_reset",
        Some("temporary password issued"),
    )
    .await?;

    Ok(())
}

pub async fn list_admin_users(pool: &PgPool) -> Result<Vec<AdminUser>, AuthError> {
    let rows = sqlx::query(
        r#"
        SELECT id, username, force_password_change
        FROM admin_user
        WHERE active = TRUE
        ORDER BY username
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AdminUser {
                id: row.try_get("id")?,
                username: row.try_get("username")?,
                force_password_change: row.try_get("force_password_change")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AuthError::from)
}

pub async fn record_audit_event(
    pool: &PgPool,
    actor_admin_id: Option<i64>,
    target_admin_id: Option<i64>,
    event_type: &str,
    detail: Option<&str>,
) -> Result<(), AuthError> {
    sqlx::query(
        r#"
        INSERT INTO admin_audit_event (
            actor_admin_user_id,
            target_admin_user_id,
            event_type,
            detail
        )
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(actor_admin_id)
    .bind(target_admin_id)
    .bind(event_type)
    .bind(detail)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn audit_event_count(pool: &PgPool, event_type: &str) -> Result<i64, AuthError> {
    sqlx::query_scalar("SELECT COUNT(*) FROM admin_audit_event WHERE event_type = $1")
        .bind(event_type)
        .fetch_one(pool)
        .await
        .map_err(AuthError::from)
}

pub async fn load_current_admin(
    pool: &PgPool,
    session_token: &str,
    idle_timeout_seconds: i64,
) -> Result<Option<CurrentAdmin>, AuthError> {
    let token_hash = hash_token(session_token);

    let row = sqlx::query(
        r#"
        SELECT au.id, au.username, au.force_password_change
        FROM admin_session s
        INNER JOIN admin_user au ON au.id = s.admin_user_id
        WHERE s.session_token_hash = $1
          AND s.revoked_at IS NULL
          AND s.expires_at > NOW()
          AND s.last_seen_at > NOW() - make_interval(secs => $2)
          AND au.active = TRUE
        "#,
    )
    .bind(&token_hash)
    .bind(idle_timeout_seconds)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    sqlx::query(
        r#"
        UPDATE admin_session
        SET last_seen_at = NOW()
        WHERE session_token_hash = $1
        "#,
    )
    .bind(&token_hash)
    .execute(pool)
    .await?;

    Ok(Some(CurrentAdmin {
        user: AdminUser {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
            force_password_change: row.try_get("force_password_change")?,
        },
        session_token_hash: token_hash,
    }))
}

pub async fn revoke_all_other_sessions(
    pool: &PgPool,
    admin_user_id: i64,
    keep_session_token_hash: Option<&str>,
) -> Result<(), AuthError> {
    match keep_session_token_hash {
        Some(token_hash) => {
            sqlx::query(
                r#"
                UPDATE admin_session
                SET revoked_at = NOW()
                WHERE admin_user_id = $1
                  AND revoked_at IS NULL
                  AND session_token_hash <> $2
                "#,
            )
            .bind(admin_user_id)
            .bind(token_hash)
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query(
                r#"
                UPDATE admin_session
                SET revoked_at = NOW()
                WHERE admin_user_id = $1
                  AND revoked_at IS NULL
                "#,
            )
            .bind(admin_user_id)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_bool_env(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn password_hasher() -> Argon2<'static> {
    let params = Params::new(19_456, 2, 1, None).expect("argon2 params should be valid");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    to_hex(&hasher.finalize())
}

fn random_hex(bytes: usize) -> String {
    let mut data = vec![0_u8; bytes];
    rand::rng().fill_bytes(&mut data);
    to_hex(&data)
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{
        MIN_PASSWORD_LENGTH, PasswordResetMode, hash_password, random_hex, to_hex, verify_password,
    };

    #[test]
    fn hashes_and_verifies_passwords() {
        let password = "correct horse battery staple";
        let hash = hash_password(password).expect("hash should succeed");

        assert!(verify_password(&hash, password));
        assert!(!verify_password(&hash, "wrong password"));
    }

    #[test]
    fn rejects_short_passwords() {
        let short_password = "x".repeat(MIN_PASSWORD_LENGTH - 1);

        assert!(hash_password(&short_password).is_err());
    }

    #[test]
    fn creates_hex_tokens() {
        let token = random_hex(32);
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn reset_mode_prefers_forced_password_change() {
        assert_eq!(
            PasswordResetMode::from_query_value(Some("admin-reset"), true),
            PasswordResetMode::Change
        );
        assert_eq!(
            PasswordResetMode::from_query_value(Some("change"), false),
            PasswordResetMode::Change
        );
        assert_eq!(
            PasswordResetMode::from_query_value(None, false),
            PasswordResetMode::AdminReset
        );
    }

    #[test]
    fn hex_encoder_matches_expected_output() {
        assert_eq!(to_hex(&[0x12, 0xab, 0xff]), "12abff");
    }
}
