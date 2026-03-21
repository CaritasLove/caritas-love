// fluent_keys.rs
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

mod common;

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    path::Path,
};

use common::{discover_locale_dirs, join_keys, load_locale_keys};

#[test]
fn fluent_keys_are_present_in_every_locale() {
    let locales_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    let locale_dirs = discover_locale_dirs(&locales_dir);

    assert!(
        !locale_dirs.is_empty(),
        "no locale directories found under {}",
        locales_dir.display()
    );

    let mut all_keys = BTreeSet::new();
    let mut locale_keys = BTreeMap::new();
    let mut duplicate_keys = BTreeMap::new();

    for locale_dir in locale_dirs {
        let locale = locale_dir
            .file_name()
            .and_then(OsStr::to_str)
            .expect("locale directory name should be valid UTF-8")
            .to_string();

        let (keys, duplicates) = load_locale_keys(&locale_dir);

        all_keys.extend(keys.iter().cloned());
        locale_keys.insert(locale.clone(), keys);

        if !duplicates.is_empty() {
            duplicate_keys.insert(locale, duplicates);
        }
    }

    let mut failures = Vec::new();

    for (locale, duplicates) in &duplicate_keys {
        failures.push(format!(
            "{locale} defines duplicate keys: {}",
            join_keys(duplicates)
        ));
    }

    for (locale, keys) in &locale_keys {
        let missing = all_keys.difference(keys).cloned().collect::<BTreeSet<_>>();

        if !missing.is_empty() {
            failures.push(format!("{locale} is missing keys: {}", join_keys(&missing)));
        }
    }

    assert!(
        failures.is_empty(),
        "locale Fluent keys are inconsistent:\n{}",
        failures.join("\n")
    );
}
