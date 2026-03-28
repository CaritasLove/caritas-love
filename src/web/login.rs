// login.rs
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

use askama::Template;
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use sqlx::Row;
use template_check_derive::CheckTemplate;

use crate::{AppState, auth, filters, web::Locale};

#[derive(Deserialize, Default)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(CheckTemplate, Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub current_path: String,
    pub locale: Locale,
    pub username: String,
    pub login_failed: bool,
    pub login_succeeded: bool,
}

impl IntoResponse for LoginTemplate {
    fn into_response(self) -> Response {
        Html(self.render().expect("template render failed")).into_response()
    }
}

impl LoginTemplate {
    fn blank(locale: Locale) -> Self {
        Self {
            current_path: "/login".to_string(),
            locale,
            username: String::new(),
            login_failed: false,
            login_succeeded: false,
        }
    }
}

pub async fn login_handler(locale: Locale) -> impl IntoResponse {
    LoginTemplate::blank(locale)
}

pub async fn login_submit(
    State(state): State<AppState>,
    locale: Locale,
    Form(form): Form<LoginForm>,
) -> Response {
    let username = form.username.trim().to_string();

    let maybe_admin = sqlx::query(
        r#"
        SELECT password_salt, password_hash, password_iterations
        FROM admin_user
        WHERE username = $1
        "#,
    )
    .bind(&username)
    .fetch_optional(&state.db)
    .await
    .expect("admin query failed");

    let is_authenticated = maybe_admin
        .map(|admin| {
            let salt: String = admin.get("password_salt");
            let hash: String = admin.get("password_hash");
            let iterations: i32 = admin.get("password_iterations");

            auth::verify_password(&form.password, &salt, &hash, iterations)
        })
        .unwrap_or(false);

    let template = LoginTemplate {
        current_path: "/login".to_string(),
        locale,
        username,
        login_failed: !is_authenticated,
        login_succeeded: is_authenticated,
    };

    let status = if is_authenticated {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    };

    (status, template).into_response()
}
