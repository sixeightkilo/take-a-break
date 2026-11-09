use crate::ui_handler::Rule;
use serde::{Deserialize, Serialize};

pub fn default_break_duration() -> u64 {
    1
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub rules: Vec<Rule>,
    #[serde(default = "default_break_duration")]
    pub break_duration: u64,
}

impl Settings {
    pub fn parse(contents: &str) -> Self {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Stored {
            Current(Settings),
            Legacy(Vec<Rule>),
        }
        let mut settings = match serde_json::from_str(contents) {
            Ok(Stored::Current(settings)) => settings,
            Ok(Stored::Legacy(rules)) => Self {
                rules,
                break_duration: 1,
            },
            Err(_) => Self {
                rules: Vec::new(),
                break_duration: 1,
            },
        };
        if !(1..=5).contains(&settings.break_duration) {
            settings.break_duration = 1;
        }
        settings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_legacy_rules_and_defaults_duration() {
        let settings =
            Settings::parse(r#"[{"days":["Mon"],"from":9,"to":17,"interval":30,"serial":1}]"#);
        assert_eq!(settings.rules.len(), 1);
        assert_eq!(settings.rules[0].interval, 30);
        assert_eq!(settings.break_duration, 1);
    }
    #[test]
    fn round_trip_and_validate_duration() {
        for duration in 1..=5 {
            let settings = Settings {
                rules: vec![],
                break_duration: duration,
            };
            assert_eq!(
                Settings::parse(&serde_json::to_string(&settings).unwrap()).break_duration,
                duration
            );
        }
        assert_eq!(
            Settings::parse(r#"{"rules":[],"break_duration":6}"#).break_duration,
            1
        );
        assert_eq!(Settings::parse(r#"{"rules":[]}"#).break_duration, 1);
    }
}
