// prod.rs
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

use super::DynLanguage;
use crate::lang::{
    BundleMap, FALLBACK_LOCALE, Language, LanguageDB, load_bundles_from_disk, resolve_locale_from,
};
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

pub(crate) struct ProductionLanguageDB {
    bundles: BundleMap,
}

impl ProductionLanguageDB {
    pub fn new() -> Self {
        let bundles = load_bundles_from_disk()
            .unwrap_or_else(|err| panic!("failed to initialize production locales: {}", err));

        Self { bundles }
    }
}

impl LanguageDB for ProductionLanguageDB {
    fn get_language(&self, requested: &LanguageIdentifier) -> DynLanguage {
        let locale = if self.bundles.contains_key(requested) {
            requested
        } else {
            &FALLBACK_LOCALE
        };

        let bundle = self.bundles.get(locale).unwrap_or_else(|| {
            panic!(
                "production locale map is missing fallback locale '{}'",
                FALLBACK_LOCALE
            )
        });

        Arc::new(Language::new(locale, bundle.clone()))
    }

    fn resolve_locale(&self, requested: &str) -> LanguageIdentifier {
        let available_locales = self.bundles.keys().cloned().collect::<Vec<_>>();
        resolve_locale_from(requested, &available_locales)
    }
}
