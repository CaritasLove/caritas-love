// demo_mode.rs
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DemoMode(pub bool);

impl DemoMode {
    pub fn from_system() -> Self {
        match std::env::var("DEMO_MODE") {
            Ok(raw) => Self::from_env_var(Some(raw.as_str())),
            Err(_) => Self(false),
        }
    }

    pub fn from_env_var(raw: Option<&str>) -> Self {
        let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
            return Self(false);
        };

        Self(matches!(
            raw.to_ascii_lowercase().as_str(),
            "1" | "true" | "t" | "yes" | "y" | "on"
        ))
    }

    pub fn is_enabled(self) -> bool {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::DemoMode;

    #[test]
    fn missing_or_empty_values_default_to_false() {
        assert!(!DemoMode::from_env_var(None).is_enabled());
        assert!(!DemoMode::from_env_var(Some("")).is_enabled());
        assert!(!DemoMode::from_env_var(Some("   ")).is_enabled());
    }

    #[test]
    fn recognized_truthy_values_are_case_insensitive() {
        assert!(DemoMode::from_env_var(Some("1")).is_enabled());
        assert!(DemoMode::from_env_var(Some("TRUE")).is_enabled());
        assert!(DemoMode::from_env_var(Some("Yes")).is_enabled());
        assert!(DemoMode::from_env_var(Some("on")).is_enabled());
    }

    #[test]
    fn falsey_and_unknown_values_are_false() {
        assert!(!DemoMode::from_env_var(Some("0")).is_enabled());
        assert!(!DemoMode::from_env_var(Some("false")).is_enabled());
        assert!(!DemoMode::from_env_var(Some("off")).is_enabled());
        assert!(!DemoMode::from_env_var(Some("demo")).is_enabled());
    }
}
