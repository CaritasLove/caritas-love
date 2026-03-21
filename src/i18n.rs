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

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::cookie::CookieJar;
use fluent::concurrent::FluentBundle;
use fluent::{FluentArgs, FluentResource};
use once_cell::sync::Lazy;
use std::{collections::HashMap, fs, sync::Arc};
use unic_langid::{LanguageIdentifier, langid};

use crate::app_env::AppEnv;

static DEFAULT_LOCALE: Lazy<LanguageIdentifier> = Lazy::new(|| langid!("en-US"));

static SUPPORTED_LOCALES: Lazy<Vec<LanguageIdentifier>> =
    Lazy::new(|| vec![langid!("en-US"), langid!("es-US")]);

type BundleMap = HashMap<LanguageIdentifier, Arc<FluentBundle<FluentResource>>>;

static PRODUCTION_BUNDLES: Lazy<Arc<BundleMap>> = Lazy::new(|| Arc::new(load_bundles()));

fn load_bundles() -> BundleMap {
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
    app_env: AppEnv,
    bundles: Arc<BundleMap>,
}

impl I18n {
    pub fn new(requested: &str, app_env: AppEnv) -> Self {
        let locale = resolve_locale(requested);
        let bundles = bundles_for_env(app_env);
        Self {
            locale,
            app_env,
            bundles,
        }
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
        let bundle = self
            .bundles
            .get(&self.locale)
            .or_else(|| self.bundles.get(&DEFAULT_LOCALE))
            .expect("default locale bundle missing");

        let message = match bundle.get_message(id) {
            Some(message) => message,
            None => {
                if self.app_env.is_development() {
                    eprintln!(
                        "missing translation key '{}' for locale '{}'",
                        id, self.locale
                    );
                }
                return format!("??{}??", id);
            }
        };

        let pattern = match message.value() {
            Some(value) => value,
            None => {
                if self.app_env.is_development() {
                    eprintln!(
                        "translation key '{}' for locale '{}' has no message value",
                        id, self.locale
                    );
                }
                return format!("??{}??", id);
            }
        };

        let mut errors = Vec::new();
        let result = bundle.format_pattern(pattern, args, &mut errors);

        if self.app_env.is_development() && !errors.is_empty() {
            eprintln!(
                "translation formatting errors for key '{}': {:?}",
                id, errors
            );
        }

        result.into_owned()
    }
}

fn bundles_for_env(app_env: AppEnv) -> Arc<BundleMap> {
    match app_env {
        AppEnv::Production => PRODUCTION_BUNDLES.clone(),
        AppEnv::Development => Arc::new(load_bundles()),
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
    AppEnv: FromRef<S>,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);

        let lang = jar
            .get("lang")
            .map(|c| c.value().to_string())
            .unwrap_or_else(|| "en-US".to_string());
        let app_env = AppEnv::from_ref(state);

        Ok(Locale(I18n::new(&lang, app_env)))
    }
}

#[cfg(test)]
mod tests {
    use super::bundles_for_env;
    use crate::app_env::AppEnv;

    #[test]
    fn production_uses_shared_cached_bundles() {
        let first = bundles_for_env(AppEnv::Production);
        let second = bundles_for_env(AppEnv::Production);

        assert!(std::sync::Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn development_reloads_bundles_for_each_request_scope() {
        let first = bundles_for_env(AppEnv::Development);
        let second = bundles_for_env(AppEnv::Development);

        assert!(!std::sync::Arc::ptr_eq(&first, &second));
    }
}
