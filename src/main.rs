// main.rs
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

mod filters;
mod i18n;
mod web;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct AppState {
    db: PgPool,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    let app_state = AppState { db: pool };

    let app = Router::new()
        .route("/hello", get(web::hello::hello_handler))
        .route(
            "/preferences/language",
            post(web::preferences::set_language),
        )
        .nest_service("/static", ServeDir::new("static"))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind failed");

    println!("Listening on http://127.0.0.1:3000");

    axum::serve(listener, app).await.expect("server failed");
}
