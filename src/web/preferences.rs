// preferences.rs
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

use axum::{
    extract::Form,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;

use crate::i18n::{is_supported_locale, resolve_locale};

#[derive(Deserialize)]
pub struct LanguageForm {
    pub lang: String,
    pub return_to: String,
}

pub async fn set_language(jar: CookieJar, Form(form): Form<LanguageForm>) -> impl IntoResponse {
    let locale = if is_supported_locale(&form.lang) {
        resolve_locale(&form.lang).to_string()
    } else {
        "en-US".to_string()
    };

    let cookie = Cookie::build(("lang", locale))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();

    let jar = jar.add(cookie);

    let return_to = sanitize_return_to(&form.return_to);

    (jar, Redirect::to(&return_to))
}

fn sanitize_return_to(return_to: &str) -> String {
    if return_to.starts_with('/') && !return_to.starts_with("//") {
        return_to.to_string()
    } else {
        "/hello".to_string()
    }
}
