// lang.rs
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

mod dev;
mod prod;

use fluent::concurrent::FluentBundle;
use fluent::{FluentArgs, FluentResource};
use log::error;
use std::{collections::HashMap, fs, sync::Arc};
use unic_langid::{LanguageIdentifier, langid};

use crate::app_env::AppEnv;

pub static FALLBACK_LOCALE: LanguageIdentifier = langid!("en-US");

pub(crate) type BundleMap = HashMap<LanguageIdentifier, Arc<FluentBundle<FluentResource>>>;

pub type DynLanguage = Arc<Language>;
pub type DynLanguageDB = Arc<dyn LanguageDB>;

pub struct Language {
    locale: LanguageIdentifier,
    bundle: Arc<FluentBundle<FluentResource>>,
}

impl Language {
    pub fn new(locale: &LanguageIdentifier, bundle: Arc<FluentBundle<FluentResource>>) -> Self {
        Self {
            locale: locale.clone(),
            bundle: bundle.clone(),
        }
    }

    pub fn locale(&self) -> LanguageIdentifier {
        self.locale.clone()
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
        let message = match self.bundle.get_message(id) {
            Some(message) => message,
            None => {
                error!(
                    "missing translation key '{}' for locale '{}'",
                    id, self.locale
                );
                return format!("??{}??", id);
            }
        };

        let pattern = match message.value() {
            Some(value) => value,
            None => {
                error!(
                    "translation key '{}' for locale '{}' has no message value",
                    id, self.locale
                );
                return format!("??{}??", id);
            }
        };

        let mut errors = Vec::new();
        let result = self.bundle.format_pattern(pattern, args, &mut errors);

        if !errors.is_empty() {
            error!(
                "translation formatting errors for key '{}': {:?}",
                id, errors
            );
        }

        result.into_owned()
    }
}

pub trait LanguageDB: Send + Sync {
    fn get_language(&self, requested: &LanguageIdentifier) -> DynLanguage;
    fn resolve_locale(&self, requested: &str) -> LanguageIdentifier;
}

pub fn load_locales(app_env: AppEnv) -> DynLanguageDB {
    match app_env {
        AppEnv::Production => Arc::new(prod::ProductionLanguageDB::new()),
        AppEnv::Development => Arc::new(dev::DevelopmentLanguageDB::new()),
    }
}

pub(crate) fn load_bundle_from_disk(
    locale: &LanguageIdentifier,
) -> Result<Arc<FluentBundle<FluentResource>>, String> {
    let locale_dir = format!("locales/{locale}");
    let entries = fs::read_dir(&locale_dir)
        .map_err(|err| format!("failed to read locale directory '{locale_dir}': {err}"))?;

    let mut paths = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|kind| kind.is_file())
                .map(|_| entry.path())
        })
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("ftl"))
        .collect::<Vec<_>>();

    paths.sort();

    if paths.is_empty() {
        return Err(format!(
            "no .ftl files found in locale directory '{locale_dir}'"
        ));
    }

    let mut bundle = FluentBundle::new_concurrent(vec![locale.clone()]);

    for path in paths {
        let source = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {}", path.display(), err))?;

        let resource = FluentResource::try_new(source)
            .map_err(|err| format!("failed to parse {}: {:?}", path.display(), err))?;

        bundle
            .add_resource(resource)
            .map_err(|err| format!("failed to add resource {}: {:?}", path.display(), err))?;
    }

    Ok(Arc::new(bundle))
}

pub(crate) fn load_bundles_from_disk() -> Result<BundleMap, String> {
    let mut bundles = BundleMap::new();

    for locale in locales_on_disk() {
        let bundle = load_bundle_from_disk(&locale)
            .map_err(|err| format!("failed to load locale '{}': {}", locale, err))?;
        bundles.insert(locale, bundle);
    }

    if !bundles.contains_key(&FALLBACK_LOCALE) {
        return Err(format!(
            "fallback locale '{}' is not available on disk",
            FALLBACK_LOCALE
        ));
    }

    Ok(bundles)
}

pub(crate) fn resolve_locale_from(
    requested: &str,
    available_locales: &[LanguageIdentifier],
) -> LanguageIdentifier {
    let requested = requested.trim();

    let Ok(parsed) = requested.parse::<LanguageIdentifier>() else {
        return FALLBACK_LOCALE.clone();
    };

    if let Some(exact) = available_locales.iter().find(|locale| **locale == parsed) {
        return exact.clone();
    }

    let requested_primary = primary_language(requested);
    let mut sorted_locales = available_locales.to_vec();
    sorted_locales.sort_by_key(|locale| locale.to_string());

    if let Some(partial) = sorted_locales
        .iter()
        .find(|locale| primary_language(&locale.to_string()) == requested_primary)
    {
        return partial.clone();
    }

    FALLBACK_LOCALE.clone()
}

pub(crate) fn locales_on_disk() -> Vec<LanguageIdentifier> {
    let Ok(entries) = fs::read_dir("locales") else {
        return vec![FALLBACK_LOCALE.clone()];
    };

    let mut locales = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|kind| kind.is_dir())
                .map(|_| entry)
        })
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|name| name.parse::<LanguageIdentifier>().ok())
        .collect::<Vec<_>>();

    locales.sort_by_key(|locale| locale.to_string());
    locales
}

fn primary_language(tag: &str) -> String {
    tag.split('-').next().unwrap_or("").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{
        FALLBACK_LOCALE, Language, dev::DevelopmentLanguageDB, load_bundle_from_disk,
        load_bundles_from_disk, prod::ProductionLanguageDB, resolve_locale_from,
    };
    use crate::lang::LanguageDB;
    use unic_langid::langid;

    #[test]
    fn resolves_exact_match() {
        let available = vec![langid!("en-US"), langid!("es-US")];

        assert_eq!(resolve_locale_from("es-US", &available), langid!("es-US"));
    }

    #[test]
    fn resolves_primary_language_fallback() {
        let available = vec![langid!("en-US"), langid!("es-US")];

        assert_eq!(resolve_locale_from("es-MX", &available), langid!("es-US"));
    }

    #[test]
    fn resolves_bare_language_fallback() {
        let available = vec![langid!("en-US"), langid!("es-US")];

        assert_eq!(resolve_locale_from("es", &available), langid!("es-US"));
    }

    #[test]
    fn falls_back_for_invalid_locale_strings() {
        let available = vec![langid!("en-US"), langid!("es-US")];

        assert_eq!(
            resolve_locale_from("not-a-locale", &available),
            FALLBACK_LOCALE
        );
    }

    #[test]
    fn falls_back_for_unsupported_valid_locales() {
        let available = vec![langid!("en-US"), langid!("es-US")];

        assert_eq!(resolve_locale_from("fr-CA", &available), FALLBACK_LOCALE);
    }

    #[test]
    fn primary_language_match_is_deterministic() {
        let available = vec![langid!("es-US"), langid!("es-419"), langid!("en-US")];

        assert_eq!(resolve_locale_from("es-MX", &available), langid!("es-419"));
    }

    #[test]
    fn exact_match_is_still_exact() {
        let available = vec![
            langid!("es-US"),
            langid!("es-419"),
            langid!("en-US"),
            langid!("es-MX"),
        ];

        assert_eq!(resolve_locale_from("es-MX", &available), langid!("es-MX"));
    }

    #[test]
    fn loads_known_locale_bundle_from_disk() {
        let bundle = load_bundle_from_disk(&langid!("en-US")).expect("failed to load en-US bundle");
        let language = Language::new(&langid!("en-US"), bundle);

        assert_eq!(language.tr("site-title"), "Caritas Admin");
    }

    #[test]
    fn development_get_language_falls_back_to_default_locale() {
        let db = DevelopmentLanguageDB::new();
        let language = db.get_language(&langid!("fr-CA"));

        assert_eq!(language.locale(), FALLBACK_LOCALE);
        assert_eq!(language.tr("site-title"), "Caritas Admin");
    }

    #[test]
    fn loads_all_bundles_from_disk_including_fallback() {
        let bundles = load_bundles_from_disk().expect("failed to load bundles from disk");

        assert!(bundles.contains_key(&langid!("en-US")));
        assert!(bundles.contains_key(&langid!("es-US")));
        assert!(bundles.contains_key(&FALLBACK_LOCALE));
    }

    #[test]
    fn production_get_language_uses_cached_fallback_locale() {
        let db = ProductionLanguageDB::new();
        let language = db.get_language(&langid!("fr-CA"));

        assert_eq!(language.locale(), FALLBACK_LOCALE);
        assert_eq!(language.tr("site-title"), "Caritas Admin");
    }
}
