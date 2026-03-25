// auth_flow.rs
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

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use caritas_love::{
    AppState,
    app_env::AppEnv,
    auth::{self, AuthConfig},
    build_app, lang,
};
use sqlx::{PgPool, Row};
use tower::util::ServiceExt;

struct TestApp {
    admin_pool: PgPool,
    pool: PgPool,
    router: Router,
    db_name: String,
}

impl TestApp {
    async fn new() -> Option<Self> {
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL").ok()?;
        let (admin_url, app_url, db_name) = derive_test_database_urls(&database_url)?;

        let admin_pool = PgPool::connect(&admin_url).await.ok()?;
        sqlx::query(&format!(r#"CREATE DATABASE "{db_name}""#))
            .execute(&admin_pool)
            .await
            .ok()?;

        let pool = PgPool::connect(&app_url).await.ok()?;
        auth::run_migrations(&pool).await.ok()?;

        let auth_config = AuthConfig {
            bootstrap_admin_username: Some(String::from("admin")),
            bootstrap_admin_password: Some(String::from("BootstrapPass123")),
            cookie_secure: false,
        };

        auth::bootstrap_admin_if_needed(&pool, &auth_config)
            .await
            .ok()?;

        let state = AppState {
            app_env: AppEnv::Development,
            auth_config,
            db: pool.clone(),
            lang: lang::load_locales(AppEnv::Development),
        };

        Some(Self {
            admin_pool,
            pool,
            router: build_app(state),
            db_name,
        })
    }

    async fn cleanup(self) {
        drop(self.router);
        self.pool.close().await;
        let _ = sqlx::query(&format!(r#"DROP DATABASE IF EXISTS "{}""#, self.db_name))
            .execute(&self.admin_pool)
            .await;
        self.admin_pool.close().await;
    }

    async fn request(&self, request: Request<Body>) -> axum::response::Response {
        self.router
            .clone()
            .oneshot(request)
            .await
            .expect("request should succeed")
    }
}

#[tokio::test]
async fn bootstrap_admin_is_only_created_once() {
    let Some(test_app) = TestApp::new().await else {
        return;
    };

    let second_config = AuthConfig {
        bootstrap_admin_username: Some(String::from("different-admin")),
        bootstrap_admin_password: Some(String::from("DifferentPass123")),
        cookie_secure: false,
    };

    auth::bootstrap_admin_if_needed(&test_app.pool, &second_config)
        .await
        .expect("second bootstrap should be ignored");

    let row = sqlx::query("SELECT COUNT(*) AS count, MIN(username) AS username FROM admin_user")
        .fetch_one(&test_app.pool)
        .await
        .expect("query should succeed");

    let admin_count: i64 = row.try_get("count").expect("count should decode");
    let username: Option<String> = row.try_get("username").expect("username should decode");

    assert_eq!(admin_count, 1);
    assert_eq!(username.as_deref(), Some("admin"));

    test_app.cleanup().await;
}

#[tokio::test]
async fn admin_login_sets_cookie_and_forces_password_change() {
    let Some(test_app) = TestApp::new().await else {
        return;
    };

    let response = test_app
        .request(admin_login_request("admin", "BootstrapPass123"))
        .await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok()),
        Some("/admin/password/reset?mode=change")
    );

    let cookie = session_cookie(&response);
    assert!(cookie.starts_with("admin_session="));

    let audit_count = auth::audit_event_count(&test_app.pool, "admin_login_success")
        .await
        .expect("audit count query should succeed");
    assert_eq!(audit_count, 1);

    test_app.cleanup().await;
}

#[tokio::test]
async fn invalid_password_is_generic_and_audited() {
    let Some(test_app) = TestApp::new().await else {
        return;
    };

    let response = test_app
        .request(admin_login_request("admin", "wrong-password"))
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should collect");
    let body = String::from_utf8(body.to_vec()).expect("body should be utf-8");

    assert!(body.contains("The username or password was not accepted."));
    assert!(!body.contains("admin not found"));

    let audit_count = auth::audit_event_count(&test_app.pool, "admin_login_failure")
        .await
        .expect("audit count query should succeed");
    assert_eq!(audit_count, 1);

    test_app.cleanup().await;
}

#[tokio::test]
async fn protected_admin_route_redirects_when_unauthenticated() {
    let Some(test_app) = TestApp::new().await else {
        return;
    };

    let response = test_app
        .request(
            Request::builder()
                .uri("/admin")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok()),
        Some("/login/admin")
    );

    test_app.cleanup().await;
}

#[tokio::test]
async fn logout_revokes_session() {
    let Some(test_app) = TestApp::new().await else {
        return;
    };

    let login_response = test_app
        .request(admin_login_request("admin", "BootstrapPass123"))
        .await;
    let session_cookie = session_cookie(&login_response);

    let response = test_app
        .request(
            Request::builder()
                .method("POST")
                .uri("/logout")
                .header(header::COOKIE, session_cookie)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok()),
        Some("/login/admin")
    );

    let revoked_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM admin_session WHERE revoked_at IS NOT NULL")
            .fetch_one(&test_app.pool)
            .await
            .expect("query should succeed");

    assert_eq!(revoked_count, 1);

    let audit_count = auth::audit_event_count(&test_app.pool, "admin_logout")
        .await
        .expect("audit count query should succeed");
    assert_eq!(audit_count, 1);

    test_app.cleanup().await;
}

#[tokio::test]
async fn admin_reset_forces_password_change_and_revokes_prior_sessions() {
    let Some(test_app) = TestApp::new().await else {
        return;
    };

    sqlx::query("UPDATE admin_user SET force_password_change = FALSE WHERE username = 'admin'")
        .execute(&test_app.pool)
        .await
        .expect("admin update should succeed");

    let target_hash = auth::hash_password("VolunteerPass123").expect("hash should succeed");
    let target_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO admin_user (username, password_hash, force_password_change, active)
        VALUES ($1, $2, FALSE, TRUE)
        RETURNING id
        "#,
    )
    .bind("second-admin")
    .bind(target_hash)
    .fetch_one(&test_app.pool)
    .await
    .expect("insert should succeed");

    let _target_session = auth::create_admin_session(&test_app.pool, target_id)
        .await
        .expect("session should be created");

    let login_response = test_app
        .request(admin_login_request("admin", "BootstrapPass123"))
        .await;
    let session_cookie = session_cookie(&login_response);

    let response = test_app
        .request(
            Request::builder()
                .method("POST")
                .uri("/admin/password/reset")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, session_cookie)
                .body(Body::from(format!(
                    "mode=admin-reset&target_admin_id={target_id}&temporary_password=TempResetPass123&new_password=TempResetPass123&confirm_password=TempResetPass123"
                )))
                .expect("request should build"),
        )
        .await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok()),
        Some("/admin/password/reset?mode=admin-reset&reset=1")
    );

    let row = sqlx::query("SELECT force_password_change FROM admin_user WHERE id = $1")
        .bind(target_id)
        .fetch_one(&test_app.pool)
        .await
        .expect("query should succeed");
    let force_password_change: bool = row
        .try_get("force_password_change")
        .expect("flag should decode");
    assert!(force_password_change);

    let revoked_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_session WHERE admin_user_id = $1 AND revoked_at IS NOT NULL",
    )
    .bind(target_id)
    .fetch_one(&test_app.pool)
    .await
    .expect("query should succeed");
    assert_eq!(revoked_count, 1);

    let audit_count = auth::audit_event_count(&test_app.pool, "admin_password_reset")
        .await
        .expect("audit count query should succeed");
    assert_eq!(audit_count, 1);

    let target_login_response = test_app
        .request(admin_login_request("second-admin", "TempResetPass123"))
        .await;

    assert_eq!(target_login_response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        target_login_response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok()),
        Some("/admin/password/reset?mode=change")
    );

    test_app.cleanup().await;
}

fn admin_login_request(username: &str, password: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/login/admin")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!(
            "username={username}&password={password}"
        )))
        .expect("request should build")
}

fn session_cookie(response: &axum::response::Response) -> String {
    response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .expect("session cookie should be present")
        .to_string()
}

fn derive_test_database_urls(database_url: &str) -> Option<(String, String, String)> {
    let (base, query_suffix) = match database_url.split_once('?') {
        Some((base, query)) => (base, format!("?{query}")),
        None => (database_url, String::new()),
    };

    let slash_index = base.rfind('/')?;
    let prefix = &base[..=slash_index];
    let database_name = format!(
        "caritas_test_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos()
    );

    Some((
        format!("{prefix}postgres{query_suffix}"),
        format!("{prefix}{database_name}{query_suffix}"),
        database_name,
    ))
}
