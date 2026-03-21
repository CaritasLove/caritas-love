# AGENTS.md

This file captures repository-specific guidance for coding agents working in
`caritas-love`.

## Project Context

- This is an Axum + Askama + Fluent application.
- Localized strings live under `locales/<locale>/*.ftl`.
- Templates live under `templates/` and may grow subdirectories over time.

## Localization Expectations

- Treat `en-US` as the canonical locale for key presence checks.
- Keep Fluent keys congruent across locales: if a key exists in one locale, it
  should exist in every locale.
- Duplicate Fluent keys within a single locale are considered errors and should
  fail tests.
- Prefer pragmatic coverage over perfect static analysis.

## Existing Localization Tests

- `tests/fluent_keys.rs`
  - Verifies all locale directories under `locales/` expose the same set of
    Fluent keys.
  - Reports missing keys per locale.
  - Reports duplicate keys within a locale.
- `tests/template_fluent_keys.rs`
  - Walks `templates/` recursively.
  - Checks literal translation usages of the form `"some-key"|tr(...)` against
    `locales/en-US`.
  - Intentionally ignores dynamic translation usages.
  - Intentionally does not report unused locale keys, because translations may
    also be used outside templates.

## Testing Preferences

- Integration tests are preferred for these localization checks.
- Shared test helpers are fine, but avoid `mod.rs` file layouts in `tests/`.
  Prefer flat filenames such as `tests/common.rs`.
- If you change locale files or template translation keys, run:
  - `cargo test --test fluent_keys --test template_fluent_keys`

## Working Preferences From The Maintainer

- The maintainer is comfortable with small pragmatic refactors when they improve
  clarity or reuse.
- When proposing additional localization checks, favor high-value, low-complexity
  checks first.
- Dynamic translation usage analysis and unused-key reporting are intentionally
  out of scope unless there is a strong reason to add them later.
