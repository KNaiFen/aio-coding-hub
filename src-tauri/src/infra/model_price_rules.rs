//! Usage: Exact model pricing overrides, independent of synchronized reference prices.

use crate::shared::fs::{read_optional_file_with_max_len, write_file_atomic};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields)]
pub struct ModelPriceItemV1 {
    pub price: Option<f64>,
    pub multiplier: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields)]
pub struct ModelPriceRuleV1 {
    pub cli_key: String,
    pub model: String,
    pub enabled: bool,
    pub multiplier: Option<f64>,
    pub input: ModelPriceItemV1,
    pub output: ModelPriceItemV1,
    pub cache_read: ModelPriceItemV1,
    pub cache_write_5m: ModelPriceItemV1,
    pub cache_write_1h: ModelPriceItemV1,
}

impl ModelPriceRuleV1 {
    pub fn items(&self) -> [&ModelPriceItemV1; 5] {
        [&self.input, &self.output, &self.cache_read, &self.cache_write_5m, &self.cache_write_1h]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields)]
pub struct ModelPriceRulesV1 {
    pub version: i64,
    pub rules: Vec<ModelPriceRuleV1>,
}

impl Default for ModelPriceRulesV1 {
    fn default() -> Self {
        Self { version: 1, rules: Vec::new() }
    }
}

impl ModelPriceRulesV1 {
    pub fn find(&self, cli_key: &str, model: &str) -> Option<&ModelPriceRuleV1> {
        self.rules.iter().find(|rule| rule.enabled && rule.cli_key == cli_key && rule.model == model)
    }

    pub fn resolve(&self, cli_key: &str, model: &str, reference_model: Option<&str>) -> Option<&ModelPriceRuleV1> {
        self.find(cli_key, model).or_else(|| reference_model.and_then(|target| self.find(cli_key, target)))
    }
}

fn validate_number(value: Option<f64>, scale: f64) -> crate::shared::error::AppResult<()> {
    if let Some(value) = value {
        if !value.is_finite() || !(0.0..=1_000_000.0).contains(&value) || (value * scale).round() / scale != value {
            return Err("SEC_INVALID_INPUT: invalid price or multiplier range/precision".to_string().into());
        }
    }
    Ok(())
}

pub fn validate(rules: &ModelPriceRulesV1) -> crate::shared::error::AppResult<()> {
    if rules.version != 1 || rules.rules.len() > 512 {
        return Err("SEC_INVALID_INPUT: invalid price rules version or count".to_string().into());
    }
    let mut keys = HashSet::new();
    for rule in &rules.rules {
        crate::shared::cli_key::validate_cli_key(&rule.cli_key)?;
        if rule.model.is_empty() || rule.model.trim() != rule.model || rule.model.len() > 200 || rule.model.contains('*') {
            return Err("SEC_INVALID_INPUT: model must be a complete name (max 200 bytes)".to_string().into());
        }
        if !keys.insert((&rule.cli_key, &rule.model)) {
            return Err("SEC_INVALID_INPUT: duplicate model price rule".to_string().into());
        }
        if rule.multiplier.is_some() && rule.items().iter().any(|item| item.multiplier.is_some()) {
            return Err("SEC_INVALID_INPUT: 整体倍率与分项倍率不能同时设置".to_string().into());
        }
        validate_number(rule.multiplier, 1_000_000.0)?;
        for item in rule.items() {
            validate_number(item.price, 1_000_000_000.0)?;
            validate_number(item.multiplier, 1_000_000.0)?;
        }
    }
    Ok(())
}

fn rules_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> crate::shared::error::AppResult<PathBuf> {
    Ok(crate::app_paths::app_data_dir(app)?.join("model-prices").join("price-rules.json"))
}

fn read_path(path: &Path) -> crate::shared::error::AppResult<ModelPriceRulesV1> {
    let Some(bytes) = read_optional_file_with_max_len(path, MAX_FILE_BYTES)? else {
        return Ok(ModelPriceRulesV1::default());
    };
    let rules = serde_json::from_slice(&bytes).map_err(|e| format!("failed to parse price rules: {e}"))?;
    validate(&rules)?;
    Ok(rules)
}

pub fn read<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> crate::shared::error::AppResult<ModelPriceRulesV1> {
    read_path(&rules_path(app)?)
}

pub fn write<R: tauri::Runtime>(app: &tauri::AppHandle<R>, rules: ModelPriceRulesV1) -> crate::shared::error::AppResult<ModelPriceRulesV1> {
    write_path(&rules_path(app)?, rules)
}

fn write_path(path: &Path, rules: ModelPriceRulesV1) -> crate::shared::error::AppResult<ModelPriceRulesV1> {
    validate(&rules)?;
    let bytes = serde_json::to_vec_pretty(&rules).map_err(|e| format!("failed to serialize price rules: {e}"))?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err("SEC_INVALID_INPUT: price rules file too large".to_string().into());
    }
    write_file_atomic(path, &bytes)?;
    Ok(rules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_conflicts_duplicates_and_unrepresentable_numbers() {
        let rule = ModelPriceRuleV1 { cli_key: "claude".into(), model: "test".into(), enabled: true, ..Default::default() };
        let mut rules = ModelPriceRulesV1 { version: 1, rules: vec![rule.clone()] };
        for value in [0.0, 1.0] {
            rules.rules[0].multiplier = Some(value);
            rules.rules[0].output.multiplier = Some(value);
            assert!(validate(&rules).is_err());
        }
        rules.rules[0].multiplier = None;
        rules.rules[0].output.price = Some(0.0);
        validate(&rules).expect("zero is valid");
        for value in [-1.0, f64::INFINITY, f64::NAN, 1_000_001.0, 0.000_000_000_1] {
            rules.rules[0].input.price = Some(value);
            assert!(validate(&rules).is_err());
        }
        rules.rules = vec![rule.clone(), rule];
        assert!(validate(&rules).is_err());
    }

    #[test]
    fn resolve_selects_one_rule_and_only_uses_a_real_reference_target() {
        let mut rules = ModelPriceRulesV1 { version: 1, rules: vec![
            ModelPriceRuleV1 { cli_key: "claude".into(), model: "source".into(), enabled: true, multiplier: Some(0.0), ..Default::default() },
            ModelPriceRuleV1 { cli_key: "claude".into(), model: "target".into(), enabled: true, multiplier: Some(2.0), ..Default::default() },
        ] };
        assert_eq!(rules.resolve("claude", "source", Some("target")).unwrap().model, "source");
        rules.rules[0].enabled = false;
        assert_eq!(rules.resolve("claude", "source", Some("target")).unwrap().model, "target");
        assert!(rules.resolve("claude", "source", None).is_none());
        assert!(rules.resolve("codex", "source", Some("target")).is_none());
    }

    #[test]
    fn invalid_file_never_reads_as_empty_rules() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("price-rules.json");
        assert!(read_path(&path).unwrap().rules.is_empty());
        std::fs::write(&path, b"invalid").unwrap();
        assert!(read_path(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"invalid");
    }

    #[test]
    fn atomic_rule_save_round_trips_and_rejected_save_preserves_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model-prices/price-rules.json");
        let rules = ModelPriceRulesV1 { version: 1, rules: vec![ModelPriceRuleV1 {
            cli_key: "claude".into(), model: "new".into(), enabled: true,
            multiplier: Some(0.0), ..Default::default()
        }] };
        write_path(&path, rules.clone()).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert_eq!(read_path(&path).unwrap().rules[0].multiplier, Some(0.0));
        let mut conflict = rules;
        conflict.rules[0].input.multiplier = Some(1.0);
        assert!(write_path(&path, conflict).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        write_path(&path, ModelPriceRulesV1::default()).unwrap();
        assert!(read_path(&path).unwrap().rules.is_empty());
    }
}
