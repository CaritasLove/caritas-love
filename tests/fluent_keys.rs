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

use fluent_syntax::{ast, parser};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

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

        let mut keys = BTreeSet::new();
        let mut duplicates = BTreeSet::new();
        let ftl_files = discover_ftl_files(&locale_dir);

        assert!(
            !ftl_files.is_empty(),
            "locale {} does not contain any .ftl files",
            locale
        );

        for file in ftl_files {
            let source = fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));

            let resource = parser::parse(source.as_str()).unwrap_or_else(|(_, errors)| {
                panic!("failed to parse {}: {:?}", file.display(), errors)
            });

            collect_resource_keys(&resource, &mut keys, &mut duplicates);
        }

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

fn discover_locale_dirs(locales_dir: &Path) -> Vec<PathBuf> {
    let mut locale_dirs = fs::read_dir(locales_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", locales_dir.display()))
        .map(|entry| entry.unwrap_or_else(|error| panic!("failed to read locale entry: {error}")))
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();

    locale_dirs.sort();
    locale_dirs
}

fn discover_ftl_files(locale_dir: &Path) -> Vec<PathBuf> {
    let mut ftl_files = fs::read_dir(locale_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", locale_dir.display()))
        .map(|entry| entry.unwrap_or_else(|error| panic!("failed to read locale entry: {error}")))
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some(OsStr::new("ftl")))
        .collect::<Vec<_>>();

    ftl_files.sort();
    ftl_files
}

fn collect_resource_keys(
    resource: &ast::Resource<&str>,
    keys: &mut BTreeSet<String>,
    duplicates: &mut BTreeSet<String>,
) {
    for entry in &resource.body {
        match entry {
            ast::Entry::Message(message) => {
                let message_id = message.id.name.to_string();
                track_key(keys, duplicates, message_id.clone());

                for attribute in &message.attributes {
                    track_key(
                        keys,
                        duplicates,
                        format!("{}.{}", message_id, attribute.id.name),
                    );
                }
            }
            ast::Entry::Term(term) => {
                let term_id = format!("-{}", term.id.name);
                track_key(keys, duplicates, term_id.clone());

                for attribute in &term.attributes {
                    track_key(
                        keys,
                        duplicates,
                        format!("{}.{}", term_id, attribute.id.name),
                    );
                }
            }
            _ => {}
        }
    }
}

fn track_key(keys: &mut BTreeSet<String>, duplicates: &mut BTreeSet<String>, key: String) {
    if !keys.insert(key.clone()) {
        duplicates.insert(key);
    }
}

fn join_keys(keys: &BTreeSet<String>) -> String {
    keys.iter().cloned().collect::<Vec<_>>().join(", ")
}
