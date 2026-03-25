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

use askama::Template;
use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use template_check_derive::CheckTemplate;

use crate::{
    AppState,
    auth::{self, AdminLoginForm, CurrentAdmin},
    filters,
    web::Locale,
};

#[derive(CheckTemplate, Template)]
#[template(path = "login_options.html")]
struct LoginOptionsTemplate {
    current_path: String,
    locale: Locale,
}

#[derive(CheckTemplate, Template)]
#[template(path = "admin_login.html")]
struct AdminLoginTemplate {
    current_path: String,
    locale: Locale,
    error: Option<String>,
}

impl IntoResponse for LoginOptionsTemplate {
    fn into_response(self) -> Response {
        Html(self.render().expect("template render failed")).into_response()
    }
}

impl IntoResponse for AdminLoginTemplate {
    fn into_response(self) -> Response {
        Html(self.render().expect("template render failed")).into_response()
    }
}

pub async fn login_options_handler(locale: Locale) -> impl IntoResponse {
    LoginOptionsTemplate {
        current_path: "/login".to_string(),
        locale,
    }
}

pub async fn admin_login_form_handler(
    locale: Locale,
    current_admin: Result<CurrentAdmin, ()>,
) -> impl IntoResponse {
    if let Ok(current_admin) = current_admin {
        if current_admin.user.force_password_change {
            return Redirect::to("/admin/password/reset?mode=change").into_response();
        }

        return Redirect::to("/admin").into_response();
    }

    AdminLoginTemplate {
        current_path: "/login/admin".to_string(),
        locale,
        error: None,
    }
    .into_response()
}

pub async fn admin_login_submit_handler(
    State(state): State<AppState>,
    locale: Locale,
    jar: CookieJar,
    Form(form): Form<AdminLoginForm>,
) -> impl IntoResponse {
    let username = form.username.trim().to_string();
    match auth::authenticate_admin(&state.db, &username, &form.password).await {
        Ok(Some(admin_user)) => {
            let session_token = auth::create_admin_session(&state.db, admin_user.id)
                .await
                .expect("session creation failed");

            auth::record_audit_event(
                &state.db,
                Some(admin_user.id),
                Some(admin_user.id),
                "admin_login_success",
                None,
            )
            .await
            .expect("audit log insert failed");

            let jar = jar.add(state.auth_config.session_cookie(session_token));
            let destination = if admin_user.force_password_change {
                "/admin/password/reset?mode=change"
            } else {
                "/admin"
            };

            (jar, Redirect::to(destination)).into_response()
        }
        Ok(None) => {
            auth::record_audit_event(
                &state.db,
                None,
                None,
                "admin_login_failure",
                Some(&format!("username={username}")),
            )
            .await
            .expect("audit log insert failed");

            AdminLoginTemplate {
                current_path: "/login/admin".to_string(),
                locale,
                error: Some(String::from("auth-invalid-credentials")),
            }
            .into_response()
        }
        Err(_) => AdminLoginTemplate {
            current_path: "/login/admin".to_string(),
            locale,
            error: Some(String::from("auth-login-error")),
        }
        .into_response(),
    }
}

pub async fn logout_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    current_admin: Result<CurrentAdmin, ()>,
) -> impl IntoResponse {
    if let Some(cookie) = jar.get(auth::ADMIN_SESSION_COOKIE) {
        let _ = auth::revoke_session_by_token(&state.db, cookie.value()).await;
    }

    if let Ok(current_admin) = current_admin {
        let _ = auth::record_audit_event(
            &state.db,
            Some(current_admin.user.id),
            Some(current_admin.user.id),
            "admin_logout",
            None,
        )
        .await;
    }

    let jar = jar.add(state.auth_config.clear_session_cookie());
    (jar, Redirect::to("/login/admin")).into_response()
}
