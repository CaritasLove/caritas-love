// common.rs
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

#![allow(dead_code)]

use fluent_syntax::{ast, parser};
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

pub fn discover_locale_dirs(locales_dir: &Path) -> Vec<PathBuf> {
    let mut locale_dirs = fs::read_dir(locales_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", locales_dir.display()))
        .map(|entry| entry.unwrap_or_else(|error| panic!("failed to read locale entry: {error}")))
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();

    locale_dirs.sort();
    locale_dirs
}

pub fn discover_ftl_files(locale_dir: &Path) -> Vec<PathBuf> {
    let mut ftl_files = fs::read_dir(locale_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", locale_dir.display()))
        .map(|entry| entry.unwrap_or_else(|error| panic!("failed to read locale entry: {error}")))
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some(OsStr::new("ftl")))
        .collect::<Vec<_>>();

    ftl_files.sort();
    ftl_files
}

pub fn load_locale_keys(locale_dir: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let ftl_files = discover_ftl_files(locale_dir);

    assert!(
        !ftl_files.is_empty(),
        "locale {} does not contain any .ftl files",
        locale_dir.display()
    );

    let mut keys = BTreeSet::new();
    let mut duplicates = BTreeSet::new();

    for file in ftl_files {
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));

        let resource = parser::parse(source.as_str()).unwrap_or_else(|(_, errors)| {
            panic!("failed to parse {}: {:?}", file.display(), errors)
        });

        collect_resource_keys(&resource, &mut keys, &mut duplicates);
    }

    (keys, duplicates)
}

pub fn discover_template_files(templates_dir: &Path) -> Vec<PathBuf> {
    let mut template_files = Vec::new();
    let mut dirs = vec![templates_dir.to_path_buf()];

    while let Some(dir) = dirs.pop() {
        let mut entries = fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()))
            .map(|entry| {
                entry.unwrap_or_else(|error| panic!("failed to read template entry: {error}"))
            })
            .map(|entry| entry.path())
            .collect::<Vec<_>>();

        entries.sort();

        for entry in entries {
            if entry.is_dir() {
                dirs.push(entry);
            } else if entry.is_file() {
                template_files.push(entry);
            }
        }
    }

    template_files.sort();
    template_files
}

pub fn join_keys(keys: &BTreeSet<String>) -> String {
    keys.iter().cloned().collect::<Vec<_>>().join(", ")
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
