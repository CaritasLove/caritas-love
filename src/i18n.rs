// i18n.rs
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

use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::cookie::CookieJar;
use fluent::concurrent::FluentBundle;
use fluent::{FluentArgs, FluentResource};
use once_cell::sync::Lazy;
use std::{collections::HashMap, fs, sync::Arc};
use unic_langid::{LanguageIdentifier, langid};

static DEFAULT_LOCALE: Lazy<LanguageIdentifier> = Lazy::new(|| langid!("en-US"));

static SUPPORTED_LOCALES: Lazy<Vec<LanguageIdentifier>> =
    Lazy::new(|| vec![langid!("en-US"), langid!("es-US")]);

static BUNDLES: Lazy<HashMap<LanguageIdentifier, Arc<FluentBundle<FluentResource>>>> =
    Lazy::new(load_bundles);

fn load_bundles() -> HashMap<LanguageIdentifier, Arc<FluentBundle<FluentResource>>> {
    let mut bundles = HashMap::new();

    for locale in SUPPORTED_LOCALES.iter() {
        let locale_dir = format!("locales/{}", locale);

        let mut bundle = FluentBundle::new_concurrent(vec![locale.clone()]);

        for file_name in ["common.ftl", "hello.ftl"] {
            let path = format!("{}/{}", locale_dir, file_name);
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {}", path, e));

            let resource = FluentResource::try_new(source)
                .unwrap_or_else(|e| panic!("failed to parse {}: {:?}", path, e));

            bundle
                .add_resource(resource)
                .unwrap_or_else(|e| panic!("failed to add resource {}: {:?}", path, e));
        }

        bundles.insert(locale.clone(), Arc::new(bundle));
    }

    bundles
}

#[derive(Clone)]
pub struct I18n {
    locale: LanguageIdentifier,
}

impl I18n {
    pub fn new(requested: &str) -> Self {
        let locale = resolve_locale(requested);
        Self { locale }
    }

    pub fn locale(&self) -> &LanguageIdentifier {
        &self.locale
    }

    pub fn locale_str(&self) -> String {
        self.locale.to_string()
    }

    pub fn tr(&self, id: &str) -> String {
        self.tr_args(id, None)
    }

    // pub fn tr_with_args<'a>(&self, id: &str, args: FluentArgs<'a>) -> String {
    //     self.tr_args(id, Some(&args))
    // }

    fn tr_args(&self, id: &str, args: Option<&FluentArgs<'_>>) -> String {
        let bundle = BUNDLES
            .get(&self.locale)
            .or_else(|| BUNDLES.get(&DEFAULT_LOCALE))
            .expect("default locale bundle missing");

        let message = match bundle.get_message(id) {
            Some(message) => message,
            None => return format!("??{}??", id),
        };

        let pattern = match message.value() {
            Some(value) => value,
            None => return format!("??{}??", id),
        };

        let mut errors = Vec::new();
        let result = bundle.format_pattern(pattern, args, &mut errors);

        if !errors.is_empty() {
            eprintln!(
                "translation formatting errors for key '{}': {:?}",
                id, errors
            );
        }

        result.into_owned()
    }
}

pub fn resolve_locale(requested: &str) -> LanguageIdentifier {
    let requested = requested.trim();

    if let Ok(parsed) = requested.parse::<LanguageIdentifier>() {
        if let Some(exact) = SUPPORTED_LOCALES.iter().find(|loc| **loc == parsed) {
            return exact.clone();
        }

        let requested_primary = primary_language(requested);
        if let Some(partial) = SUPPORTED_LOCALES
            .iter()
            .find(|loc| primary_language(&loc.to_string()) == requested_primary)
        {
            return partial.clone();
        }
    }

    DEFAULT_LOCALE.clone()
}

pub fn is_supported_locale(requested: &str) -> bool {
    let requested = requested.trim();

    if let Ok(parsed) = requested.parse::<LanguageIdentifier>() {
        return SUPPORTED_LOCALES.contains(&parsed)
            || SUPPORTED_LOCALES
                .iter()
                .any(|loc| primary_language(&loc.to_string()) == primary_language(requested));
    }

    false
}

fn primary_language(tag: &str) -> String {
    tag.split('-').next().unwrap_or("").to_ascii_lowercase()
}

#[derive(Clone)]
pub struct Locale(pub I18n);

impl<S> FromRequestParts<S> for Locale
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);

        let lang = jar
            .get("lang")
            .map(|c| c.value().to_string())
            .unwrap_or_else(|| "en-US".to_string());

        Ok(Locale(I18n::new(&lang)))
    }
}
