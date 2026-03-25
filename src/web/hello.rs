// hello.rs
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
    extract::State,
    response::{Html, IntoResponse, Response},
};
use template_check_derive::CheckTemplate;
// use serde::Deserialize;

use crate::{AppState, filters, web::Locale};

// #[derive(Deserialize, Default)]
// pub struct HelloQuery {
//     pub lang: Option<String>,
// }

#[derive(CheckTemplate, Template)]
#[template(path = "hello.html")]
pub struct HelloTemplate {
    pub current_path: String,
    pub locale: Locale,
    pub message: String,
}

impl IntoResponse for HelloTemplate {
    fn into_response(self) -> Response {
        Html(self.render().expect("template render failed")).into_response()
    }
}

pub async fn hello_handler(State(state): State<AppState>, locale: Locale) -> impl IntoResponse {
    let message: String = sqlx::query_scalar(
        r#"
        SELECT message
        FROM greeting
        WHERE id = 1
        "#,
    )
    .fetch_one(&state.db)
    .await
    .expect("query failed");

    HelloTemplate {
        locale,
        message,
        current_path: "/hello".to_string(),
    }
}
