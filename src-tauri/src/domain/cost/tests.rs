use super::*;

#[test]
fn model_rule_applies_fixed_prices_and_mutually_exclusive_multipliers() {
    let reference = r#"{"input_cost_per_token":0.000002,"output_cost_per_token":0.00001,"cache_read_input_token_cost":0.0000002}"#;
    let mut rule = crate::model_price_rules::ModelPriceRuleV1::default();
    rule.output.price = Some(8.0);
    let usages = [
        CostUsage {
            input_tokens: 1_000_000,
            ..Default::default()
        },
        CostUsage {
            output_tokens: 1_000_000,
            ..Default::default()
        },
        CostUsage {
            cache_read_input_tokens: 1_000_000,
            ..Default::default()
        },
    ];
    for (whole, output, expected) in [
        (
            Some(0.5),
            None,
            [
                800_000_000_000_000,
                3_200_000_000_000_000,
                80_000_000_000_000,
            ],
        ),
        (
            None,
            Some(2.0),
            [
                1_600_000_000_000_000,
                12_800_000_000_000_000,
                160_000_000_000_000,
            ],
        ),
        (
            None,
            None,
            [
                1_600_000_000_000_000,
                6_400_000_000_000_000,
                160_000_000_000_000,
            ],
        ),
    ] {
        rule.multiplier = whole;
        rule.output.multiplier = output;
        for (usage, expected) in usages.iter().zip(expected) {
            assert_eq!(
                calculate_cost_with_rule(
                    usage,
                    Some(reference),
                    0.8,
                    "claude",
                    "test",
                    &CostCalculationOptions::default(),
                    Some(&rule)
                ),
                Some(expected)
            );
        }
    }
}

#[test]
fn overrides_preserve_cache_reference_and_token_conventions() {
    let reference = r#"{"input_cost_per_token":0.000002,"output_cost_per_token":0.00001}"#;
    let mut rule = crate::model_price_rules::ModelPriceRuleV1::default();
    rule.input.price = Some(9.0);
    rule.cache_write_1h.price = Some(7.0);
    rule.cache_read.multiplier = Some(2.0);
    let usage = CostUsage {
        input_tokens: 1000,
        cache_read_input_tokens: 100,
        cache_creation_5m_input_tokens: 100,
        cache_creation_1h_input_tokens: 100,
        ..Default::default()
    };
    for (cli, expected) in [
        ("claude", 9_990_000_000_000),
        ("codex", 7_290_000_000_000),
        ("gemini", 9_090_000_000_000),
    ] {
        assert_eq!(
            calculate_cost_with_rule(
                &usage,
                Some(reference),
                1.0,
                cli,
                "test",
                &CostCalculationOptions::default(),
                Some(&rule)
            ),
            Some(expected)
        );
    }
}

#[test]
fn fixed_price_removes_only_its_premium_while_multiplier_preserves_tiers() {
    let reference = r#"{"input_cost_per_token":0.000002,"input_cost_per_token_priority":0.000004,"input_cost_per_token_above_200k_tokens":0.000006,"output_cost_per_token":0.00001}"#;
    let usage = CostUsage {
        input_tokens: 300_000,
        output_tokens: 300_000,
        ..Default::default()
    };
    let options = CostCalculationOptions {
        priority_service_tier_applied: true,
    };
    let mut rule = crate::model_price_rules::ModelPriceRuleV1::default();
    rule.input.multiplier = Some(2.0);
    rule.output.price = Some(8.0);
    assert_eq!(
        calculate_cost_with_rule(
            &usage,
            Some(reference),
            1.0,
            "codex",
            "test",
            &options,
            Some(&rule)
        ),
        Some(5_200_000_000_000_000)
    );
    assert_eq!(
        calculate_cost_with_rule(
            &usage,
            Some(reference),
            1.0,
            "claude",
            "test-1m",
            &options,
            Some(&rule)
        ),
        Some(5_600_000_000_000_000)
    );
}

#[test]
fn custom_new_models_require_primary_prices_and_distinguish_free_from_unknown() {
    let usage = CostUsage {
        input_tokens: 1_000,
        ..Default::default()
    };
    let options = CostCalculationOptions::default();
    let mut rule = crate::model_price_rules::ModelPriceRuleV1::default();
    rule.input.price = Some(0.0);
    assert_eq!(
        calculate_cost_with_rule(&usage, None, 1.0, "claude", "new", &options, Some(&rule)),
        None
    );
    rule.output.price = Some(0.0);
    assert_eq!(
        calculate_cost_with_rule(&usage, None, 1.0, "claude", "new", &options, Some(&rule)),
        Some(0)
    );
    assert_eq!(
        calculate_cost_with_rule(
            &CostUsage::default(),
            None,
            1.0,
            "claude",
            "new",
            &options,
            Some(&rule)
        ),
        None
    );
    rule.input.price = Some(2.0);
    rule.output.price = Some(10.0);
    let cache_usage = CostUsage {
        cache_read_input_tokens: 100,
        cache_creation_5m_input_tokens: 100,
        cache_creation_1h_input_tokens: 100,
        ..Default::default()
    };
    assert_eq!(
        calculate_cost_with_rule(
            &cache_usage,
            None,
            1.0,
            "claude",
            "new",
            &options,
            Some(&rule)
        ),
        Some(670_000_000_000)
    );
    rule.multiplier = Some(0.0);
    assert_eq!(
        calculate_cost_with_rule(&usage, None, 1.0, "claude", "new", &options, Some(&rule)),
        Some(0)
    );
    rule.input.price = None;
    rule.output.price = None;
    assert_eq!(
        calculate_cost_with_rule(
            &usage,
            Some("{}"),
            1.0,
            "claude",
            "new",
            &options,
            Some(&rule)
        ),
        None
    );
    assert_eq!(
        calculate_cost_usd_femto(
            &usage,
            r#"{"input_cost_per_token":0}"#,
            1.0,
            "claude",
            "old"
        ),
        None
    );
}

#[test]
fn empty_rule_preserves_existing_cost_and_partial_unknown_does_not_become_free() {
    let reference = r#"{"input_cost_per_token":0.000002,"output_cost_per_token":0.00001}"#;
    let usage = CostUsage {
        input_tokens: 300_000,
        output_tokens: 300_000,
        cache_read_input_tokens: 100,
        cache_creation_input_tokens: 100,
        ..Default::default()
    };
    let rule = crate::model_price_rules::ModelPriceRuleV1::default();
    for cli in ["claude", "codex", "grok", "gemini"] {
        assert_eq!(
            calculate_cost_with_rule(
                &usage,
                Some(reference),
                0.8,
                cli,
                "test-1m",
                &CostCalculationOptions::default(),
                Some(&rule)
            ),
            calculate_cost_usd_femto(&usage, reference, 0.8, cli, "test-1m")
        );
    }
    let rule = crate::model_price_rules::ModelPriceRuleV1 {
        multiplier: Some(0.0),
        ..Default::default()
    };
    assert_eq!(
        calculate_cost_with_rule(
            &usage,
            Some(r#"{"input_cost_per_token":0.000002}"#),
            1.0,
            "claude",
            "test",
            &CostCalculationOptions::default(),
            Some(&rule)
        ),
        None
    );
}

#[test]
fn parses_decimal_with_exponent_to_femto() {
    let femto = parse_decimal_to_femto("1.5e-6").expect("parse");
    // 0.0000015 * 1e15 = 1.5e9
    assert_eq!(femto, 1_500_000_000);
}

#[test]
fn calculates_basic_cost() {
    let usage = CostUsage {
        input_tokens: 10,
        output_tokens: 5,
        ..Default::default()
    };
    let price_json = r#"{"input_cost_per_token":0.01,"output_cost_per_token":0.02}"#;
    let cost = calculate_cost_usd_femto(&usage, price_json, 1.0, "codex", "gpt").expect("cost");

    let expected = (10i128 * 10_000_000_000_000i128) + (5i128 * 20_000_000_000_000i128);
    assert_eq!(cost as i128, expected);
}

#[test]
fn tiered_cost_with_separate_prices_applies_above_200k() {
    let usage = CostUsage {
        input_tokens: 200_001,
        ..Default::default()
    };
    let price_json = r#"{
      "input_cost_per_token": 0.01,
      "input_cost_per_token_above_200k_tokens": 0.02
    }"#;
    let cost =
        calculate_cost_usd_femto(&usage, price_json, 1.0, "gemini", "gemini-test").expect("cost");

    let base = 200_000i128 * 10_000_000_000_000i128;
    let premium = 20_000_000_000_000i128;
    assert_eq!(cost as i128, base + premium);
}

#[test]
fn tiered_cost_with_context_1m_multiplier_applies_for_claude_1m_model() {
    let usage = CostUsage {
        input_tokens: 200_001,
        output_tokens: 200_001,
        ..Default::default()
    };
    let price_json = r#"{
      "input_cost_per_token": 0.01,
      "output_cost_per_token": 0.02
    }"#;
    let cost =
        calculate_cost_usd_femto(&usage, price_json, 1.0, "claude", "claude-1m").expect("cost");

    let input_base = 200_000i128 * 10_000_000_000_000i128;
    let input_premium = 20_000_000_000_000i128; // 2x

    let output_base = 200_000i128 * 20_000_000_000_000i128;
    let output_premium = 30_000_000_000_000i128; // 1.5x

    assert_eq!(
        cost as i128,
        input_base + input_premium + output_base + output_premium
    );
}

#[test]
fn applies_provider_multiplier() {
    let usage = CostUsage {
        input_tokens: 10,
        ..Default::default()
    };
    let price_json = r#"{"input_cost_per_token":0.01}"#;
    let cost = calculate_cost_usd_femto(&usage, price_json, 1.5, "codex", "gpt").expect("cost");

    let base = 10i128 * 10_000_000_000_000i128;
    let expected = base.saturating_mul(1_500_000) / 1_000_000;
    assert_eq!(cost as i128, expected);
}

#[test]
fn calculates_cost_with_basellm_exponent_price_json() {
    let usage = CostUsage {
        input_tokens: 100,
        output_tokens: 20,
        cache_read_input_tokens: 50,
        cache_creation_input_tokens: 15,
        cache_creation_5m_input_tokens: 10,
        cache_creation_1h_input_tokens: 5,
    };

    let price_json = r#"{
      "cache_creation_input_token_cost":"3.75e-6",
      "cache_creation_input_token_cost_above_1hr":"3.75e-6",
      "cache_read_input_token_cost":"0.3e-6",
      "input_cost_per_token":"3e-6",
      "output_cost_per_token":"15e-6"
    }"#;

    let cost = calculate_cost_usd_femto(&usage, price_json, 1.0, "codex", "gpt").expect("cost");
    assert_eq!(cost, 476_250_000_000);
}

#[test]
fn openai_semantics_clis_do_not_double_charge_cache_read_or_creation_with_explicit_price() {
    let usage = CostUsage {
        input_tokens: 1_000,
        output_tokens: 50,
        cache_read_input_tokens: 100,
        cache_creation_input_tokens: 200,
        ..Default::default()
    };

    let price_json = r#"{
      "input_cost_per_token": 0.004,
      "output_cost_per_token": 0.02,
      "cache_read_input_token_cost": 0.001,
      "cache_creation_input_token_cost": 0.006
    }"#;

    let input = 4_000_000_000_000i128;
    let output = 20_000_000_000_000i128;
    let cache_read = 1_000_000_000_000i128;
    let cache_creation = 6_000_000_000_000i128;

    let expected =
        (700i128 * input) + (50i128 * output) + (100i128 * cache_read) + (200i128 * cache_creation);
    for (cli_key, model) in [("codex", "gpt"), ("grok", "grok-build")] {
        let cost = calculate_cost_usd_femto(&usage, price_json, 1.0, cli_key, model).expect("cost");
        assert_eq!(cost as i128, expected, "unexpected cost for {cli_key}");
    }
}

#[test]
fn codex_cache_creation_price_falls_back_to_one_point_two_five_times_input() {
    let usage = CostUsage {
        input_tokens: 1_000,
        output_tokens: 50,
        cache_read_input_tokens: 100,
        cache_creation_input_tokens: 200,
        ..Default::default()
    };

    let price_json = r#"{
      "input_cost_per_token": 0.004,
      "output_cost_per_token": 0.02,
      "cache_read_input_token_cost": 0.001
    }"#;

    let cost = calculate_cost_usd_femto(&usage, price_json, 1.0, "codex", "gpt").expect("cost");

    let input = 4_000_000_000_000i128;
    let output = 20_000_000_000_000i128;
    let cache_read = 1_000_000_000_000i128;
    let cache_creation_fallback = 5_000_000_000_000i128;

    let expected = (700i128 * input)
        + (50i128 * output)
        + (100i128 * cache_read)
        + (200i128 * cache_creation_fallback);
    assert_eq!(cost as i128, expected);
}

#[test]
fn codex_oversubscribed_cache_buckets_clamp_ordinary_input_to_zero() {
    let usage = CostUsage {
        input_tokens: 100,
        cache_read_input_tokens: 80,
        cache_creation_input_tokens: 50,
        ..Default::default()
    };

    let price_json = r#"{
      "input_cost_per_token": 0.004,
      "cache_read_input_token_cost": 0.001,
      "cache_creation_input_token_cost": 0.006
    }"#;

    let cost = calculate_cost_usd_femto(&usage, price_json, 1.0, "codex", "gpt").expect("cost");
    let expected = (80i128 * 1_000_000_000_000i128) + (50i128 * 6_000_000_000_000i128);
    assert_eq!(cost as i128, expected);
}

#[test]
fn fixed_input_price_clamps_oversubscribed_cache_buckets() {
    let reference = r#"{
      "input_cost_per_token": 0.004,
      "cache_read_input_token_cost": 0.001,
      "cache_creation_input_token_cost": 0.006
    }"#;
    let mut rule = crate::model_price_rules::ModelPriceRuleV1::default();
    rule.input.price = Some(4_000.0);
    for (cli, cache_read, expected) in [
        ("codex", 80, 380_000_000_000_000),
        ("grok", 80, 380_000_000_000_000),
        ("gemini", 130, 430_000_000_000_000),
        ("claude", 80, 780_000_000_000_000),
    ] {
        for ttl_breakdown in [false, true] {
            let usage = CostUsage {
                input_tokens: 100,
                cache_read_input_tokens: cache_read,
                cache_creation_input_tokens: 50,
                cache_creation_5m_input_tokens: if ttl_breakdown { 50 } else { 0 },
                ..Default::default()
            };
            assert_eq!(
                calculate_cost_with_rule(
                    &usage,
                    Some(reference),
                    1.0,
                    cli,
                    "test",
                    &CostCalculationOptions::default(),
                    Some(&rule),
                ),
                Some(expected),
                "unexpected fixed input cost for {cli}, TTL breakdown: {ttl_breakdown}"
            );
        }
    }
}

#[test]
fn gemini_only_subtracts_cache_read_from_input() {
    let usage = CostUsage {
        input_tokens: 100,
        output_tokens: 10,
        cache_read_input_tokens: 80,
        cache_creation_input_tokens: 10,
        ..Default::default()
    };

    let price_json = r#"{
      "input_cost_per_token": 0.01,
      "output_cost_per_token": 0.02,
      "cache_read_input_token_cost": 0.001,
      "cache_creation_input_token_cost": 0.005
    }"#;

    let cost =
        calculate_cost_usd_femto(&usage, price_json, 1.0, "gemini", "gemini-test").expect("cost");

    let input = 10_000_000_000_000i128;
    let output = 20_000_000_000_000i128;
    let cache_read = 1_000_000_000_000i128;
    let cache_creation = 5_000_000_000_000i128;

    let expected =
        (20i128 * input) + (10i128 * output) + (80i128 * cache_read) + (10i128 * cache_creation);
    assert_eq!(cost as i128, expected);
}

#[test]
fn claude_keeps_cache_buckets_additive_cost() {
    let usage = CostUsage {
        input_tokens: 100,
        cache_read_input_tokens: 80,
        cache_creation_input_tokens: 10,
        ..Default::default()
    };

    let price_json = r#"{
      "input_cost_per_token": 0.01,
      "cache_read_input_token_cost": 0.001,
      "cache_creation_input_token_cost": 0.005
    }"#;

    let cost =
        calculate_cost_usd_femto(&usage, price_json, 1.0, "claude", "claude-test").expect("cost");

    let input = 10_000_000_000_000i128;
    let cache_read = 1_000_000_000_000i128;
    let cache_creation = 5_000_000_000_000i128;

    let expected = (100i128 * input) + (80i128 * cache_read) + (10i128 * cache_creation);
    assert_eq!(cost as i128, expected);
}

#[test]
fn claude_opus_46_price_json_calculates_nonzero_cost() {
    let usage = CostUsage {
        input_tokens: 1,
        output_tokens: 134,
        cache_read_input_tokens: 86_059,
        cache_creation_5m_input_tokens: 1_700,
        ..Default::default()
    };

    let price_json = r#"{
      "cache_creation_input_token_cost":"0.00000625",
      "cache_creation_input_token_cost_above_1hr":"0.00000625",
      "cache_read_input_token_cost":"0.0000005",
      "input_cost_per_token":"0.000005",
      "input_cost_per_token_above_200k_tokens":"0.00001",
      "output_cost_per_token":"0.000025",
      "output_cost_per_token_above_200k_tokens":"0.0000375"
    }"#;

    let cost = calculate_cost_usd_femto(&usage, price_json, 1.0, "claude", "claude-opus-4-6")
        .expect("cost should be present");

    assert!(cost > 0);
}

#[test]
fn tiered_cost_keeps_non_negative_when_below_threshold() {
    let cost = tiered_cost_with_separate_prices(1, 25_000_000_000, 37_500_000_000);
    assert_eq!(cost, 25_000_000_000);
}
