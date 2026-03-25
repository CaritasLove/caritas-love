// filters.rs
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

//! Askama custom filters are looked up through a module literally named
//! `filters`.
//!
//! That convention is hardcoded in Askama's derive implementation, so renaming
//! this module will break `#[derive(Template)]` unless each template module
//! adds an alias back to `filters`. Source:
//! <https://docs.rs/crate/askama_derive/0.15.5/source/src/generator/filter.rs#L155-L160>

use crate::web::Locale;

#[askama::filter_fn]
pub fn tr(
    value: impl std::fmt::Display,
    _env: &dyn askama::Values,
    locale: &Locale,
) -> askama::Result<String> {
    Ok(locale.tr(&value.to_string()))
}
