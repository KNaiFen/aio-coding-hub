use serde::{Deserialize, Serialize};

pub const PROVIDER_MODEL_POLICY_VERSION: u32 = 1;
pub const PROVIDER_MODEL_POLICY_MAX_RULES: usize = 500;
pub const PROVIDER_MODEL_POLICY_MAX_VALUE_CHARS: usize = 200;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderModelPolicyStatus {
    Legacy,
    Ready,
    Invalid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderModelMode {
    All,
    Selected,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderModelRule {
    pub source: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderModelPolicyV1 {
    pub version: u32,
    pub mode: ProviderModelMode,
    pub rules: Vec<ProviderModelRule>,
}

impl ProviderModelPolicyV1 {
    pub fn all() -> Self {
        Self {
            version: PROVIDER_MODEL_POLICY_VERSION,
            mode: ProviderModelMode::All,
            rules: Vec::new(),
        }
    }

    pub fn normalized(mut self) -> Result<Self, String> {
        if self.version != PROVIDER_MODEL_POLICY_VERSION {
            return Err(format!(
                "SEC_INVALID_INPUT: unsupported provider model policy version {}",
                self.version
            ));
        }
        if self.rules.len() > PROVIDER_MODEL_POLICY_MAX_RULES {
            return Err(format!(
                "SEC_INVALID_INPUT: provider model policy supports at most {} rules",
                PROVIDER_MODEL_POLICY_MAX_RULES
            ));
        }

        let mut seen = std::collections::HashSet::with_capacity(self.rules.len());
        for rule in &mut self.rules {
            rule.source = rule.source.trim().to_string();
            rule.target = rule
                .target
                .take()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());

            validate_policy_value("source", &rule.source)?;
            if let Some(target) = rule.target.as_deref() {
                validate_policy_value("target", target)?;
                if !rule.source.contains('*') && target.contains('*') {
                    return Err(
                        "SEC_INVALID_INPUT: target wildcard requires a source wildcard".into(),
                    );
                }
            }
            if !seen.insert(rule.source.clone()) {
                return Err("SEC_INVALID_INPUT: provider model policy source is duplicated".into());
            }
        }
        if self.mode == ProviderModelMode::Selected && self.rules.is_empty() {
            return Err("SEC_INVALID_INPUT: selected provider model policy needs a rule".into());
        }

        self.rules.sort_by(|left, right| {
            let left_wildcard = left.source.contains('*');
            let right_wildcard = right.source.contains('*');
            left_wildcard
                .cmp(&right_wildcard)
                .then_with(|| {
                    right
                        .source
                        .chars()
                        .filter(|character| *character != '*')
                        .count()
                        .cmp(
                            &left
                                .source
                                .chars()
                                .filter(|character| *character != '*')
                                .count(),
                        )
                })
                .then_with(|| left.source.cmp(&right.source))
        });
        Ok(self)
    }

    pub fn decode(raw: Option<&str>, cli_key: &str) -> (Option<Self>, ProviderModelPolicyStatus) {
        let Some(raw) = raw else {
            return if cli_key == "claude" {
                (None, ProviderModelPolicyStatus::Legacy)
            } else {
                (Some(Self::all()), ProviderModelPolicyStatus::Ready)
            };
        };

        match serde_json::from_str::<Self>(raw)
            .map_err(|error| error.to_string())
            .and_then(Self::normalized)
        {
            Ok(policy) => (Some(policy), ProviderModelPolicyStatus::Ready),
            Err(_) => (None, ProviderModelPolicyStatus::Invalid),
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|error| format!("SYSTEM_ERROR: {error}"))
    }

    pub fn resolve(&self, source_model: &str) -> Option<String> {
        for rule in &self.rules {
            let Some(capture) = match_rule(&rule.source, source_model) else {
                continue;
            };
            return Some(resolve_target(
                rule.target.as_deref(),
                source_model,
                capture,
            ));
        }
        (self.mode == ProviderModelMode::All).then(|| source_model.to_string())
    }

    #[allow(dead_code)]
    pub fn merge_discovered_model_ids(
        &self,
        discovered_model_ids: impl IntoIterator<Item = String>,
    ) -> Result<Self, String> {
        if self.mode == ProviderModelMode::All {
            return Ok(self.clone());
        }

        let mut merged = self.clone();
        let mut known = merged
            .rules
            .iter()
            .map(|rule| rule.source.clone())
            .collect::<std::collections::HashSet<_>>();
        for model_id in discovered_model_ids {
            let model_id = model_id.trim();
            if model_id.is_empty()
                || self.resolve(model_id).is_some()
                || !known.insert(model_id.into())
            {
                continue;
            }
            merged.rules.push(ProviderModelRule {
                source: model_id.to_string(),
                target: None,
            });
        }
        merged.normalized()
    }
}

fn validate_policy_value(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!(
            "SEC_INVALID_INPUT: provider model policy {field} is required"
        ));
    }
    if value.chars().count() > PROVIDER_MODEL_POLICY_MAX_VALUE_CHARS {
        return Err(format!(
            "SEC_INVALID_INPUT: provider model policy {field} exceeds {} characters",
            PROVIDER_MODEL_POLICY_MAX_VALUE_CHARS
        ));
    }
    if value.matches('*').count() > 1 {
        return Err(format!(
            "SEC_INVALID_INPUT: provider model policy {field} supports at most one wildcard"
        ));
    }
    Ok(())
}

fn match_rule<'a>(pattern: &'a str, source_model: &'a str) -> Option<&'a str> {
    let Some(star) = pattern.find('*') else {
        return (pattern == source_model).then_some("");
    };
    let prefix = &pattern[..star];
    let suffix = &pattern[star + 1..];
    let remainder = source_model.strip_prefix(prefix)?;
    let capture = remainder.strip_suffix(suffix)?;
    Some(capture)
}

fn resolve_target(target: Option<&str>, source_model: &str, capture: &str) -> String {
    let Some(target) = target else {
        return source_model.to_string();
    };
    target.replace('*', capture)
}

#[cfg(test)]
mod tests {
    use super::{ProviderModelMode, ProviderModelPolicyV1, ProviderModelRule};

    fn rule(source: &str, target: Option<&str>) -> ProviderModelRule {
        ProviderModelRule {
            source: source.to_string(),
            target: target.map(str::to_string),
        }
    }

    fn policy(mode: ProviderModelMode, rules: Vec<ProviderModelRule>) -> ProviderModelPolicyV1 {
        ProviderModelPolicyV1 {
            version: 1,
            mode,
            rules,
        }
    }

    #[test]
    fn provider_model_policy_normalizes_and_validates_rules() {
        let normalized = policy(
            ProviderModelMode::All,
            vec![rule("  gpt-5.4  ", Some("  upstream-5.4  "))],
        )
        .normalized()
        .expect("valid policy");
        assert_eq!(
            normalized.rules,
            vec![rule("gpt-5.4", Some("upstream-5.4"))]
        );

        let invalid = [
            policy(ProviderModelMode::Selected, vec![]),
            policy(ProviderModelMode::All, vec![rule("", None)]),
            policy(
                ProviderModelMode::All,
                vec![rule("gpt-*", None), rule(" gpt-* ", None)],
            ),
            policy(ProviderModelMode::All, vec![rule("gpt-*-*", None)]),
            policy(
                ProviderModelMode::All,
                vec![rule("gpt", Some("upstream-*"))],
            ),
        ];
        for value in invalid {
            assert!(value.normalized().is_err());
        }
    }

    #[test]
    fn provider_model_policy_matches_once_with_stable_priority() {
        let value = policy(
            ProviderModelMode::Selected,
            vec![
                rule("gpt-*", Some("broad-*")),
                rule("gpt-5.*", Some("specific-*")),
                rule("gpt-5.4", Some("exact")),
                rule("gpt-5.*-mini", Some("vendor-*")),
            ],
        )
        .normalized()
        .expect("valid policy");

        assert_eq!(value.resolve("gpt-5.4"), Some("exact".to_string()));
        assert_eq!(value.resolve("gpt-5.4-mini"), Some("vendor-4".to_string()));
        assert_eq!(value.resolve("gpt-5.3"), Some("specific-3".to_string()));
        assert_eq!(value.resolve("claude-sonnet"), None);

        let non_recursive = policy(
            ProviderModelMode::Selected,
            vec![
                rule("source", Some("gpt-5.4")),
                rule("gpt-5.4", Some("final")),
            ],
        )
        .normalized()
        .expect("valid policy");
        assert_eq!(non_recursive.resolve("source"), Some("gpt-5.4".to_string()));
    }

    #[test]
    fn provider_model_policy_all_keeps_unmatched_models() {
        let value = policy(ProviderModelMode::All, vec![])
            .normalized()
            .expect("valid policy");
        assert_eq!(value.resolve("gpt-5.4"), Some("gpt-5.4".to_string()));
    }

    #[test]
    fn provider_model_policy_enforces_capacity_and_unicode_scalar_limits() {
        let max_rules = (0..500)
            .map(|index| rule(&format!("model-{index}"), None))
            .collect();
        assert!(policy(ProviderModelMode::Selected, max_rules)
            .normalized()
            .is_ok());

        let too_many_rules = (0..501)
            .map(|index| rule(&format!("model-{index}"), None))
            .collect();
        assert!(policy(ProviderModelMode::Selected, too_many_rules)
            .normalized()
            .is_err());

        assert!(policy(
            ProviderModelMode::Selected,
            vec![rule(&"模".repeat(200), Some(&"型".repeat(200)))]
        )
        .normalized()
        .is_ok());
        assert!(policy(
            ProviderModelMode::Selected,
            vec![rule(&"模".repeat(201), None)]
        )
        .normalized()
        .is_err());
    }
}
