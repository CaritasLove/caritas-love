// template_fluent_keys.rs
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

use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use common::{discover_template_files, join_keys, load_locale_keys};

#[test]
fn template_literal_translation_keys_exist_in_en_us() {
    let templates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let template_files = discover_template_files(&templates_dir);

    assert!(
        !template_files.is_empty(),
        "no template files found under {}",
        templates_dir.display()
    );

    let en_us_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("locales")
        .join("en-US");
    let (locale_keys, duplicates) = load_locale_keys(&en_us_dir);

    assert!(
        duplicates.is_empty(),
        "en-US defines duplicate keys: {}",
        join_keys(&duplicates)
    );

    let key_pattern =
        Regex::new(r#""([^"\\]+)"\s*\|\s*tr\s*\("#).expect("literal translation regex is valid");

    let mut missing_by_template = BTreeMap::new();

    for template_file in template_files {
        let source = fs::read_to_string(&template_file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", template_file.display()));

        let mut missing = BTreeSet::new();

        for captures in key_pattern.captures_iter(&source) {
            let key = captures
                .get(1)
                .expect("regex should capture the translation key")
                .as_str();

            if !locale_keys.contains(key) {
                missing.insert(key.to_string());
            }
        }

        if !missing.is_empty() {
            let relative_path = template_file
                .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
                .expect("template file should be inside the crate")
                .display()
                .to_string();

            missing_by_template.insert(relative_path, missing);
        }
    }

    let failures = missing_by_template
        .into_iter()
        .map(|(template, missing)| {
            format!(
                "{template} references missing keys: {}",
                join_keys(&missing)
            )
        })
        .collect::<Vec<_>>();

    assert!(
        failures.is_empty(),
        "template translation keys are missing from locales/en-US:\n{}",
        failures.join("\n")
    );
}
