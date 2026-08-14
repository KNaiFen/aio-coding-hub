use crate::gateway::proxy::failover::select_next_provider_id_from_order;
use crate::providers;
use std::collections::{HashMap, HashSet};

pub(super) fn reorder_providers_by_bound_order(
    providers: &mut Vec<providers::ProviderForGateway>,
    order: &[i64],
) {
    if order.is_empty() || providers.len() <= 1 {
        return;
    }

    let mut order_rank = HashMap::with_capacity(order.len());
    for (index, provider_id) in order.iter().copied().enumerate() {
        order_rank.entry(provider_id).or_insert(index);
    }

    let provider_ids: Vec<i64> = providers.iter().map(|provider| provider.id).collect();
    let mut positions_by_priority: HashMap<i64, Vec<usize>> = HashMap::new();
    for (index, provider) in providers.iter().enumerate() {
        positions_by_priority
            .entry(provider.session_reuse_priority)
            .or_default()
            .push(index);
    }

    let mut source_for_position: Vec<usize> = (0..providers.len()).collect();
    for positions in positions_by_priority.values() {
        let mut ordered_sources = positions.clone();
        ordered_sources.sort_by_key(|index| {
            order_rank
                .get(&provider_ids[*index])
                .copied()
                .unwrap_or(usize::MAX)
        });
        for (position, source) in positions.iter().zip(ordered_sources) {
            source_for_position[*position] = source;
        }
    }

    let mut original: Vec<Option<providers::ProviderForGateway>> =
        providers.drain(..).map(Some).collect();
    *providers = source_for_position
        .into_iter()
        .map(|source| {
            original[source]
                .take()
                .expect("each provider must appear once after ordering")
        })
        .collect();
}

fn has_higher_session_reuse_priority(
    providers: &[providers::ProviderForGateway],
    session_reuse_priority: i64,
) -> bool {
    providers
        .iter()
        .any(|provider| provider.session_reuse_priority > session_reuse_priority)
}

pub(super) fn apply_session_provider_preference(
    providers: &mut [providers::ProviderForGateway],
    bound_provider_id: i64,
    bound_provider_order: Option<&[i64]>,
) -> Option<i64> {
    if providers.is_empty() {
        return None;
    }

    if let Some(idx) = providers.iter().position(|p| p.id == bound_provider_id) {
        let bound_priority = providers[idx].session_reuse_priority;
        if has_higher_session_reuse_priority(providers, bound_priority) {
            // Do not turn session reuse into a second availability gate. Keeping the
            // configured route order lets the common gate record any skipped higher tier.
            return None;
        }
        if idx > 0 {
            providers.rotate_left(idx);
        }
        return Some(bound_provider_id);
    }

    let order = bound_provider_order?;
    if order.is_empty() || providers.len() <= 1 {
        return None;
    }

    let current_provider_ids: HashSet<i64> = providers.iter().map(|p| p.id).collect();
    let next_provider_id =
        select_next_provider_id_from_order(bound_provider_id, order, &current_provider_ids)?;

    if let Some(idx) = providers.iter().position(|p| p.id == next_provider_id) {
        if has_higher_session_reuse_priority(providers, providers[idx].session_reuse_priority) {
            return None;
        }
        if idx > 0 {
            providers.rotate_left(idx);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{apply_session_provider_preference, reorder_providers_by_bound_order};
    use crate::providers;

    fn provider(id: i64) -> providers::ProviderForGateway {
        providers::ProviderForGateway {
            id,
            provider_uuid: format!("00000000-0000-4000-8000-{id:012}"),
            session_reuse_priority: 0,
            name: format!("p{id}"),
            base_urls: vec!["https://example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            api_key_plaintext: String::new(),
            claude_models: providers::ClaudeModels::default(),
            model_mapping: Default::default(),
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: providers::DailyResetMode::Fixed,
            daily_reset_time: "00:00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
            extension_values: vec![],
            upstream_retry_policy_override: None,
            model_routing_policy_override: None,
            cross_provider_model_routing_policy: None,
        }
    }

    fn ids(items: &[providers::ProviderForGateway]) -> Vec<i64> {
        items.iter().map(|item| item.id).collect()
    }

    #[test]
    fn reorder_by_bound_order_preserves_unspecified_tail() {
        let mut providers = vec![provider(1), provider(2), provider(3), provider(4)];
        reorder_providers_by_bound_order(&mut providers, &[3, 1]);
        assert_eq!(ids(&providers), vec![3, 1, 2, 4]);
    }

    #[test]
    fn reorder_by_bound_order_does_not_promote_lower_priority_members() {
        let mut providers = vec![provider(1), provider(2)];
        providers[0].session_reuse_priority = 100;

        reorder_providers_by_bound_order(&mut providers, &[2, 1]);

        assert_eq!(ids(&providers), vec![1, 2]);
    }

    #[test]
    fn apply_session_preference_rotates_from_bound_provider_when_present() {
        let mut providers = vec![provider(11), provider(22), provider(33)];
        let selected = apply_session_provider_preference(&mut providers, 22, Some(&[11, 22, 33]));
        assert_eq!(selected, Some(22));
        assert_eq!(ids(&providers), vec![22, 33, 11]);
    }

    #[test]
    fn apply_session_preference_keeps_route_order_for_lower_priority_binding() {
        let mut providers = vec![provider(11), provider(22), provider(33)];
        providers[0].session_reuse_priority = 100;

        let selected = apply_session_provider_preference(&mut providers, 22, Some(&[11, 22, 33]));

        assert_eq!(selected, None);
        assert_eq!(ids(&providers), vec![11, 22, 33]);
    }

    #[test]
    fn apply_session_preference_rotates_to_next_when_bound_missing() {
        let mut providers = vec![provider(10), provider(20), provider(30)];
        let selected = apply_session_provider_preference(&mut providers, 99, Some(&[99, 30, 20]));
        assert_eq!(selected, None);
        assert_eq!(ids(&providers), vec![30, 10, 20]);
    }

    #[test]
    fn apply_session_preference_does_not_promote_lower_priority_fallback() {
        let mut providers = vec![provider(10), provider(20)];
        providers[0].session_reuse_priority = 100;

        let selected = apply_session_provider_preference(&mut providers, 99, Some(&[20, 10]));

        assert_eq!(selected, None);
        assert_eq!(ids(&providers), vec![10, 20]);
    }

    #[test]
    fn apply_session_preference_is_noop_without_bound_order() {
        let mut providers = vec![provider(10), provider(20), provider(30)];
        let selected = apply_session_provider_preference(&mut providers, 99, None);
        assert_eq!(selected, None);
        assert_eq!(ids(&providers), vec![10, 20, 30]);
    }
}
