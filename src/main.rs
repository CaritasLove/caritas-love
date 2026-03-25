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

use caritas_love::{AppState, app_env::AppEnv, auth, build_app, lang, logging};
use log::{debug, error, info, trace, warn};
use sqlx::PgPool;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let app_host = std::env::var("APP_HOST").unwrap_or_else(|_| String::from("127.0.0.1"));
    let app_port = std::env::var("APP_PORT").unwrap_or_else(|_| String::from("3000"));
    let listen_addr = format!("{app_host}:{app_port}");
    let app_env = AppEnv::from_system();
    let auth_config = auth::AuthConfig::from_system(app_env);

    let _log_handle = logging::init_logging("./log").expect("Failed to initialize logging");
    info!("caritas-love starting");
    debug!("debug logging enabled");
    warn!("this warning goes to file and stderr");
    error!("this error goes to file and stderr");
    trace!("very detailed trace output");

    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    auth::run_migrations(&db)
        .await
        .expect("Failed to run database migrations");
    auth::bootstrap_admin_if_needed(&db, &auth_config)
        .await
        .expect("Failed to bootstrap admin account");

    let lang = lang::load_locales(app_env);

    let app_state = AppState {
        app_env,
        auth_config,
        db,
        lang,
    };

    let app = build_app(app_state);

    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .expect("bind failed");

    info!("Listening on http://{}", listen_addr);

    axum::serve(listener, app).await.expect("server failed");
}
