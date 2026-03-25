// admin.rs
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
    extract::{Form, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use template_check_derive::CheckTemplate;

use crate::{
    AppState,
    auth::{self, CurrentAdmin, PasswordChangeForm, PasswordResetMode},
    filters,
    web::Locale,
};

#[derive(Clone)]
struct AdminArea {
    href: &'static str,
    title_key: &'static str,
    body_key: &'static str,
}

#[derive(Clone)]
struct AdminUserRow {
    id: i64,
    username: String,
    force_password_change: bool,
}

#[derive(CheckTemplate, Template)]
#[template(path = "admin_dashboard.html")]
struct AdminDashboardTemplate {
    current_path: String,
    locale: Locale,
    admin_username: String,
    areas: Vec<AdminArea>,
}

#[derive(CheckTemplate, Template)]
#[template(path = "admin_password_reset.html")]
struct AdminPasswordResetTemplate {
    current_path: String,
    locale: Locale,
    admin_username: String,
    current_admin_id: i64,
    force_password_change: bool,
    error: Option<String>,
    success: Option<String>,
    admins: Vec<AdminUserRow>,
}

#[derive(Deserialize)]
pub struct PasswordResetQuery {
    changed: Option<String>,
    reset: Option<String>,
}

impl IntoResponse for AdminDashboardTemplate {
    fn into_response(self) -> Response {
        Html(self.render().expect("template render failed")).into_response()
    }
}

impl IntoResponse for AdminPasswordResetTemplate {
    fn into_response(self) -> Response {
        Html(self.render().expect("template render failed")).into_response()
    }
}

pub async fn dashboard_handler(
    locale: Locale,
    current_admin: Result<CurrentAdmin, ()>,
) -> impl IntoResponse {
    let Ok(current_admin) = current_admin else {
        return Redirect::to("/login/admin").into_response();
    };

    if current_admin.user.force_password_change {
        return Redirect::to("/admin/password/reset?mode=change").into_response();
    }

    AdminDashboardTemplate {
        current_path: "/admin".to_string(),
        locale,
        admin_username: current_admin.user.username,
        areas: admin_areas(),
    }
    .into_response()
}

pub async fn password_reset_form_handler(
    State(state): State<AppState>,
    locale: Locale,
    current_admin: Result<CurrentAdmin, ()>,
    Query(query): Query<PasswordResetQuery>,
) -> impl IntoResponse {
    let Ok(current_admin) = current_admin else {
        return Redirect::to("/login/admin").into_response();
    };

    let admins = auth::list_admin_users(&state.db)
        .await
        .expect("admin list query failed")
        .into_iter()
        .map(|user| AdminUserRow {
            id: user.id,
            username: user.username,
            force_password_change: user.force_password_change,
        })
        .collect::<Vec<_>>();

    AdminPasswordResetTemplate {
        current_path: "/admin/password/reset".to_string(),
        locale,
        admin_username: current_admin.user.username,
        current_admin_id: current_admin.user.id,
        force_password_change: current_admin.user.force_password_change,
        error: None,
        success: success_message_key(&query),
        admins,
    }
    .into_response()
}

pub async fn password_reset_submit_handler(
    State(state): State<AppState>,
    locale: Locale,
    current_admin: Result<CurrentAdmin, ()>,
    Form(form): Form<PasswordChangeForm>,
) -> impl IntoResponse {
    let Ok(current_admin) = current_admin else {
        return Redirect::to("/login/admin").into_response();
    };

    let admins = auth::list_admin_users(&state.db)
        .await
        .expect("admin list query failed")
        .into_iter()
        .map(|user| AdminUserRow {
            id: user.id,
            username: user.username,
            force_password_change: user.force_password_change,
        })
        .collect::<Vec<_>>();

    let mode = PasswordResetMode::from_query_value(
        Some(form.mode.as_str()),
        current_admin.user.force_password_change,
    );

    if form.new_password != form.confirm_password {
        return AdminPasswordResetTemplate {
            current_path: "/admin/password/reset".to_string(),
            locale,
            admin_username: current_admin.user.username,
            current_admin_id: current_admin.user.id,
            force_password_change: current_admin.user.force_password_change,
            error: Some(String::from("auth-password-mismatch")),
            success: None,
            admins,
        }
        .into_response();
    }

    let result = match mode {
        PasswordResetMode::Change => {
            let current_password = form.current_password.as_deref().unwrap_or_default();
            let change_result = auth::change_own_password(
                &state.db,
                &current_admin.user,
                &current_admin.session_token_hash,
                current_password,
                &form.new_password,
            )
            .await;

            if change_result.is_ok() {
                auth::record_audit_event(
                    &state.db,
                    Some(current_admin.user.id),
                    Some(current_admin.user.id),
                    "admin_password_changed",
                    None,
                )
                .await
            } else {
                change_result
            }
        }
        PasswordResetMode::AdminReset => {
            if current_admin.user.force_password_change {
                return Redirect::to("/admin/password/reset?mode=change").into_response();
            }

            let temporary_password = form.temporary_password.as_deref().unwrap_or_default();
            auth::admin_reset_password(
                &state.db,
                current_admin.user.id,
                form.target_admin_id,
                temporary_password,
            )
            .await
        }
    };

    match result {
        Ok(()) => {
            let destination = match mode {
                PasswordResetMode::Change => "/admin/password/reset?mode=change&changed=1",
                PasswordResetMode::AdminReset => "/admin/password/reset?mode=admin-reset&reset=1",
            };
            Redirect::to(destination).into_response()
        }
        Err(auth::AuthError::InvalidCredentials) => AdminPasswordResetTemplate {
            current_path: "/admin/password/reset".to_string(),
            locale,
            admin_username: current_admin.user.username,
            current_admin_id: current_admin.user.id,
            force_password_change: current_admin.user.force_password_change,
            error: Some(String::from("auth-invalid-credentials")),
            success: None,
            admins,
        }
        .into_response(),
        Err(auth::AuthError::PasswordTooShort) => AdminPasswordResetTemplate {
            current_path: "/admin/password/reset".to_string(),
            locale,
            admin_username: current_admin.user.username,
            current_admin_id: current_admin.user.id,
            force_password_change: current_admin.user.force_password_change,
            error: Some(String::from("auth-password-too-short")),
            success: None,
            admins,
        }
        .into_response(),
        Err(_) => AdminPasswordResetTemplate {
            current_path: "/admin/password/reset".to_string(),
            locale,
            admin_username: current_admin.user.username,
            current_admin_id: current_admin.user.id,
            force_password_change: current_admin.user.force_password_change,
            error: Some(String::from("auth-reset-error")),
            success: None,
            admins,
        }
        .into_response(),
    }
}

fn admin_areas() -> Vec<AdminArea> {
    vec![
        AdminArea {
            href: "#registration",
            title_key: "admin-area-registration-title",
            body_key: "admin-area-registration-body",
        },
        AdminArea {
            href: "#inventory",
            title_key: "admin-area-inventory-title",
            body_key: "admin-area-inventory-body",
        },
        AdminArea {
            href: "#volunteer-records",
            title_key: "admin-area-volunteer-records-title",
            body_key: "admin-area-volunteer-records-body",
        },
        AdminArea {
            href: "#reporting",
            title_key: "admin-area-reporting-title",
            body_key: "admin-area-reporting-body",
        },
        AdminArea {
            href: "/admin/password/reset",
            title_key: "admin-area-settings-title",
            body_key: "admin-area-settings-body",
        },
    ]
}

fn success_message_key(query: &PasswordResetQuery) -> Option<String> {
    if query.changed.as_deref() == Some("1") {
        return Some(String::from("auth-password-change-success"));
    }

    if query.reset.as_deref() == Some("1") {
        return Some(String::from("auth-password-reset-success"));
    }

    None
}
