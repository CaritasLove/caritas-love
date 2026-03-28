// web.rs
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

pub mod hello;
pub mod login;
pub mod preferences;
pub mod request_ip;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::CookieJar;
use unic_langid::LanguageIdentifier;

use crate::lang::{DynLanguage, DynLanguageDB, FALLBACK_LOCALE};

#[derive(Clone)]
pub struct Locale(pub DynLanguage);

impl<S> FromRequestParts<S> for Locale
where
    S: Send + Sync,
    DynLanguageDB: FromRef<S>,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);

        let lang = jar
            .get("lang")
            .map(|c| c.value().to_string())
            .unwrap_or_else(|| FALLBACK_LOCALE.to_string());

        let langdb = DynLanguageDB::from_ref(state);

        let lang_id = (*langdb).resolve_locale(&lang);
        let locale = (*langdb).get_language(&lang_id);

        Ok(Locale(locale))
    }
}

impl Locale {
    pub fn tr(&self, id: &str) -> String {
        self.0.tr(id)
    }

    pub fn locale(&self) -> LanguageIdentifier {
        self.0.locale()
    }

    pub fn locale_str(&self) -> String {
        self.0.locale_str()
    }
}
