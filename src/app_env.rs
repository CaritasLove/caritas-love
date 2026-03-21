// app_env.rs
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
pub enum AppEnv {
    Production,
    Development,
}

impl AppEnv {
    pub fn from_system() -> Self {
        match std::env::var("APP_ENV") {
            Ok(raw) => Self::from_env_var(Some(raw.as_str())),
            Err(_) => Self::Production,
        }
    }

    pub fn from_env_var(raw: Option<&str>) -> Self {
        let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
            return Self::Production;
        };

        match raw.to_ascii_lowercase().as_str() {
            "production" | "prod" => Self::Production,
            "development" | "dev" => Self::Development,
            _ => {
                eprintln!(
                    "unknown APP_ENV value '{}'; falling back to production mode",
                    raw
                );
                Self::Production
            }
        }
    }

    pub fn is_development(self) -> bool {
        self == Self::Development
    }
}

#[cfg(test)]
mod tests {
    use super::AppEnv;

    #[test]
    fn missing_or_empty_values_default_to_production() {
        assert_eq!(AppEnv::from_env_var(None), AppEnv::Production);
        assert_eq!(AppEnv::from_env_var(Some("")), AppEnv::Production);
        assert_eq!(AppEnv::from_env_var(Some("   ")), AppEnv::Production);
    }

    #[test]
    fn recognized_values_are_case_insensitive() {
        assert_eq!(AppEnv::from_env_var(Some("production")), AppEnv::Production);
        assert_eq!(AppEnv::from_env_var(Some("PROD")), AppEnv::Production);
        assert_eq!(
            AppEnv::from_env_var(Some("development")),
            AppEnv::Development
        );
        assert_eq!(AppEnv::from_env_var(Some("DeV")), AppEnv::Development);
    }

    #[test]
    fn unknown_values_fall_back_to_production() {
        assert_eq!(AppEnv::from_env_var(Some("staging")), AppEnv::Production);
    }
}
