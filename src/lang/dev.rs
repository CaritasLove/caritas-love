// dev.rs
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
    FALLBACK_LOCALE, Language, LanguageDB, load_bundle_from_disk, locales_on_disk,
    resolve_locale_from,
};
use log::error;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

pub(crate) struct DevelopmentLanguageDB;

impl DevelopmentLanguageDB {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageDB for DevelopmentLanguageDB {
    fn get_language(&self, requested: &LanguageIdentifier) -> DynLanguage {
        match load_bundle_from_disk(requested) {
            Ok(bundle) => Arc::new(Language::new(requested, bundle)),
            Err(err) if *requested != FALLBACK_LOCALE => {
                error!(
                    "failed to load locale '{}': {}; falling back to '{}'",
                    requested, err, FALLBACK_LOCALE
                );

                let fallback_bundle =
                    load_bundle_from_disk(&FALLBACK_LOCALE).unwrap_or_else(|fallback_err| {
                        panic!(
                            "failed to load fallback locale '{}': {}",
                            FALLBACK_LOCALE, fallback_err
                        )
                    });

                Arc::new(Language::new(&FALLBACK_LOCALE, fallback_bundle))
            }
            Err(err) => {
                panic!(
                    "failed to load fallback locale '{}': {}",
                    FALLBACK_LOCALE, err
                )
            }
        }
    }

    fn resolve_locale(&self, requested: &str) -> LanguageIdentifier {
        let available_locales = locales_on_disk();
        resolve_locale_from(requested, &available_locales)
    }
}
