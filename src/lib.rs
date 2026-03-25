// lib.rs
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

pub mod app_env;
pub mod auth;
pub mod filters;
pub mod lang;
pub mod logging;
pub mod template;
pub mod web;

use axum::{
    Router,
    extract::FromRef,
    routing::{get, post},
};
use sqlx::PgPool;
use tower_http::services::ServeDir;

use crate::{app_env::AppEnv, auth::AuthConfig, lang::DynLanguageDB};

#[derive(Clone)]
pub struct AppState {
    pub app_env: AppEnv,
    pub auth_config: AuthConfig,
    pub db: PgPool,
    pub lang: DynLanguageDB,
}

impl FromRef<AppState> for AppEnv {
    fn from_ref(state: &AppState) -> Self {
        state.app_env
    }
}

impl FromRef<AppState> for AuthConfig {
    fn from_ref(state: &AppState) -> Self {
        state.auth_config.clone()
    }
}

impl FromRef<AppState> for DynLanguageDB {
    fn from_ref(state: &AppState) -> Self {
        state.lang.clone()
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

pub fn build_app(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(web::root::root_handler))
        .route("/hello", get(web::hello::hello_handler))
        .route("/login", get(web::auth::login_options_handler))
        .route(
            "/login/admin",
            get(web::auth::admin_login_form_handler).post(web::auth::admin_login_submit_handler),
        )
        .route("/logout", post(web::auth::logout_handler))
        .route("/admin", get(web::admin::dashboard_handler))
        .route(
            "/admin/password/reset",
            get(web::admin::password_reset_form_handler)
                .post(web::admin::password_reset_submit_handler),
        )
        .route(
            "/preferences/language",
            post(web::preferences::set_language),
        )
        .nest_service("/static", ServeDir::new("static"))
        .with_state(app_state)
}
