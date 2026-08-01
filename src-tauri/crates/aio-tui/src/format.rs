use aio_observer_protocol::{CliScope, ObserverRequest, ObserverRequestState, ObserverSnapshotV1};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

pub fn cli_label(cli_key: &str) -> &str {
    match cli_key.trim().to_ascii_lowercase().as_str() {
        "claude" => "Claude",
        "codex" => "Codex",
        "grok" => "Grok",
        "gemini" => "Gemini",
        "all" => "全部",
        _ => "未知",
    }
}

pub fn scope_label(scope: CliScope) -> &'static str {
    cli_label(scope.as_str())
}

pub fn status_segments(snapshot: &ObserverSnapshotV1) -> Vec<String> {
    let preferred = if !snapshot.preferred_provider.available {
        "首选 不可用".to_string()
    } else if let Some(provider) = snapshot.preferred_provider.value.as_ref() {
        if snapshot.scope == CliScope::All {
            format!(
                "首选 {}/{}",
                cli_label(&provider.cli_key),
                provider.provider_name
            )
        } else {
            format!("首选 {}", provider.provider_name)
        }
    } else {
        "首选 —".to_string()
    };
    let last = if !snapshot.last_request.available {
        "上次 不可用".to_string()
    } else if let Some(request) = snapshot.last_request.value.as_ref() {
        format!(
            "上次 {} {} {}",
            terminal_status(request),
            request.provider_name.as_deref().unwrap_or("—"),
            route_result(request)
        )
    } else {
        "上次 —".to_string()
    };
    let dominant = if !snapshot.dominant_provider.available {
        "近10 不可用".to_string()
    } else if let Some(provider) = snapshot.dominant_provider.value.as_ref() {
        format!("近10 {}×{}", provider.provider_name, provider.count)
    } else {
        "近10 —".to_string()
    };
    let today_cost = if !snapshot.today.available {
        "今日 不可用".to_string()
    } else {
        format!(
            "今日 {}",
            snapshot
                .today
                .value
                .as_ref()
                .and_then(|value| value.cost_usd)
                .map(format_cost)
                .unwrap_or_else(|| "—".to_string())
        )
    };
    let today_tokens = if !snapshot.today.available {
        "Token 不可用".to_string()
    } else {
        format!(
            "Token {}",
            snapshot
                .today
                .value
                .as_ref()
                .map(|value| format_tokens(value.total_tokens))
                .unwrap_or_else(|| "—".to_string())
        )
    };
    vec![
        preferred,
        last,
        dominant,
        format!("并发 {}", snapshot.active_inference_count),
        today_cost,
        today_tokens,
    ]
}

pub fn status_plain(snapshot: &ObserverSnapshotV1) -> String {
    status_segments(snapshot).join(" | ")
}

pub fn wrap_status_segments(segments: &[String], width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for segment in segments {
        let pieces = wrap_display(segment, width);
        for (index, piece) in pieces.into_iter().enumerate() {
            if index > 0 {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                lines.push(piece);
                continue;
            }
            let separator = if current.is_empty() { "" } else { " | " };
            let candidate_width = display_width(&current)
                .saturating_add(display_width(separator))
                .saturating_add(display_width(&piece));
            if candidate_width <= width {
                current.push_str(separator);
                current.push_str(&piece);
            } else {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                current = piece;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub fn request_card_lines(request: &ObserverRequest, now_ms: i64, width: usize) -> [String; 5] {
    let compaction = request
        .context_compaction
        .as_ref()
        .map(|marker| match marker.mode.as_str() {
            "local" => " 压缩·本地",
            "remote" => " 压缩·远程",
            _ => " 压缩·未知",
        })
        .unwrap_or_default();
    let model = request.model.as_deref().unwrap_or("—");
    let folder = request.folder_name.as_deref().unwrap_or("无目录");
    let provider = request.provider_name.as_deref().unwrap_or("未上游");
    let duration = request
        .duration_ms
        .map(format_duration)
        .unwrap_or_else(|| "—".to_string());
    let (input, output, cache) = request
        .usage
        .as_ref()
        .map(|usage| {
            (
                usage
                    .input_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string()),
                usage
                    .output_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string()),
                usage
                    .cache_read_tokens
                    .unwrap_or(0)
                    .saturating_add(usage.cache_creation_tokens.unwrap_or(0)),
            )
        })
        .unwrap_or_else(|| ("—".to_string(), "—".to_string(), 0));
    let cost = request
        .cost_usd
        .map(format_cost)
        .unwrap_or_else(|| "—".to_string());
    [
        truncate_display(
            &format!(
                "{}  {}",
                request_status_label(request),
                relative_time(request.created_at_ms, now_ms)
            ),
            width,
        ),
        truncate_display(
            &format!("{} / {}{}", cli_label(&request.cli_key), model, compaction),
            width,
        ),
        truncate_display(&format!("{}  {}", folder, provider), width),
        truncate_display(&format!("{}  {}", route_result(request), duration), width),
        truncate_display(
            &format!(
                "I {} O {} C {} {}",
                input,
                output,
                format_tokens(cache),
                cost
            ),
            width,
        ),
    ]
}

pub fn detail_lines(request: &ObserverRequest, now_ms: i64) -> Vec<String> {
    let mut lines = vec![
        format!("状态  {}", request_status_label(request)),
        format!("时间  {}", relative_time(request.created_at_ms, now_ms)),
        format!("CLI   {}", cli_label(&request.cli_key)),
        format!("模型  {}", request.model.as_deref().unwrap_or("—")),
        format!("目录  {}", request.folder_name.as_deref().unwrap_or("—")),
        format!(
            "供应商  {}",
            request.provider_name.as_deref().unwrap_or("—")
        ),
        format!("路由  {}", route_result(request)),
        format!("方法  {} {}", request.method, request.path),
        format!(
            "耗时  {}",
            request
                .duration_ms
                .map(format_duration)
                .unwrap_or_else(|| "—".to_string())
        ),
        format!(
            "首字  {}",
            request
                .ttfb_ms
                .map(format_duration)
                .unwrap_or_else(|| "—".to_string())
        ),
        format!(
            "尝试  {}（重试 {}）",
            request.attempt_count, request.retry_count
        ),
        format!("错误码  {}", request.error_code.as_deref().unwrap_or("—")),
        format!("Session  {}", request.session_id.as_deref().unwrap_or("—")),
    ];
    if let Some(usage) = request.usage.as_ref() {
        lines.extend([
            format!(
                "输入  {}",
                usage
                    .input_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string())
            ),
            format!(
                "输出  {}",
                usage
                    .output_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string())
            ),
            format!(
                "缓存读  {}",
                usage
                    .cache_read_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string())
            ),
            format!(
                "缓存写  {}",
                usage
                    .cache_creation_tokens
                    .map(format_tokens)
                    .unwrap_or_else(|| "—".to_string())
            ),
        ]);
    }
    lines.push(format!(
        "费用  {}",
        request
            .cost_usd
            .map(format_cost)
            .unwrap_or_else(|| "—".to_string())
    ));
    if let Some(marker) = request.context_compaction.as_ref() {
        lines.extend([
            String::new(),
            "上下文压缩".to_string(),
            format!("模式  {}", marker.mode),
            format!("实现  {}", marker.implementation),
            format!("触发  {}", marker.trigger),
            format!("原因  {}", marker.reason),
            format!("阶段  {}", marker.phase),
            format!("策略  {}", marker.strategy),
        ]);
    }
    if !request.route.is_empty() {
        lines.push(String::new());
        lines.push("路由链".to_string());
        for (index, hop) in request.route.iter().enumerate() {
            lines.push(format!(
                "{}. {} ×{} {} {}",
                index + 1,
                hop.provider_name,
                hop.attempts,
                hop.status
                    .map(|status| status.to_string())
                    .unwrap_or_else(|| "—".to_string()),
                hop.error_code.as_deref().unwrap_or("")
            ));
        }
    }
    lines
}

pub fn request_status_label(request: &ObserverRequest) -> String {
    match request.state {
        ObserverRequestState::Active => "进行中".to_string(),
        ObserverRequestState::Terminal if request.interrupted => "499 已中断".to_string(),
        ObserverRequestState::Terminal => match request.status {
            Some(status) if (200..300).contains(&status) => format!("{status} 成功"),
            Some(status) => format!("{status} 失败"),
            None if request.error_code.is_some() => "请求失败".to_string(),
            None => "状态未知".to_string(),
        },
    }
}

pub fn route_result(request: &ObserverRequest) -> String {
    if request.provider_switch_count > 0 {
        if request.retry_count > request.provider_switch_count {
            return format!(
                "切换{}/重试{}",
                request.provider_switch_count, request.retry_count
            );
        }
        return format!("切换{}", request.provider_switch_count);
    }
    if request.retry_count > 0 {
        return format!("重试{}", request.retry_count);
    }
    if request.attempt_count > 0 {
        return "直连".to_string();
    }
    "未上游".to_string()
}

pub fn terminal_status(request: &ObserverRequest) -> String {
    if request.interrupted {
        return "499".to_string();
    }
    request
        .status
        .map(|status| status.to_string())
        .unwrap_or_else(|| "ERR".to_string())
}

pub fn format_duration(milliseconds: i64) -> String {
    let milliseconds = milliseconds.max(0);
    if milliseconds < 1000 {
        return format!("{milliseconds}ms");
    }
    if milliseconds < 60_000 {
        return format!("{:.1}s", milliseconds as f64 / 1000.0);
    }
    let minutes = milliseconds / 60_000;
    let seconds = (milliseconds % 60_000) / 1000;
    format!("{minutes}m{seconds:02}s")
}

pub fn format_tokens(value: i64) -> String {
    let value = value.max(0) as f64;
    if value >= 1_000_000_000.0 {
        format_compact(value / 1_000_000_000.0, "B")
    } else if value >= 1_000_000.0 {
        format_compact(value / 1_000_000.0, "M")
    } else if value >= 1_000.0 {
        format_compact(value / 1_000.0, "K")
    } else {
        format!("{}", value as i64)
    }
}

fn format_compact(value: f64, suffix: &str) -> String {
    if value >= 100.0 {
        format!("{value:.0}{suffix}")
    } else if value >= 10.0 {
        format!("{value:.1}{suffix}")
    } else {
        format!("{value:.2}{suffix}")
    }
}

pub fn format_cost(value: f64) -> String {
    if value >= 100.0 {
        format!("${value:.2}")
    } else if value >= 1.0 {
        format!("${value:.3}")
    } else {
        format!("${value:.6}")
    }
}

pub fn relative_time(created_at_ms: i64, now_ms: i64) -> String {
    let elapsed = now_ms.saturating_sub(created_at_ms).max(0);
    if elapsed < 60_000 {
        return "<1分钟".to_string();
    }
    if elapsed < 3_600_000 {
        return format!("{}分钟", elapsed / 60_000);
    }
    if elapsed < 86_400_000 {
        return format!("{}小时", elapsed / 3_600_000);
    }
    format!("{}天", elapsed / 86_400_000)
}

pub fn truncate_display(value: &str, max_width: usize) -> String {
    if display_width(value) <= max_width {
        return value.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "…".to_string();
    }
    let target = max_width - 1;
    let mut out = String::new();
    let mut width = 0;
    for grapheme in value.graphemes(true) {
        let next = grapheme.width();
        if width.saturating_add(next) > target {
            break;
        }
        out.push_str(grapheme);
        width += next;
    }
    out.push('…');
    out
}

fn wrap_display(value: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut width = 0;
    for grapheme in value.graphemes(true) {
        let next = grapheme.width();
        if !current.is_empty() && width.saturating_add(next) > max_width {
            lines.push(std::mem::take(&mut current));
            width = 0;
        }
        current.push_str(grapheme);
        width = width.saturating_add(next);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_respects_cjk_display_width() {
        assert_eq!(truncate_display("供应商OpenAI", 8), "供应商O…");
        assert!(display_width(&truncate_display("供应商OpenAI", 8)) <= 8);
    }

    #[test]
    fn status_segments_wrap_at_boundaries_first() {
        let lines = wrap_status_segments(
            &[
                "首选 Provider".to_string(),
                "并发 13".to_string(),
                "今日 $1.23".to_string(),
            ],
            16,
        );
        assert_eq!(lines, vec!["首选 Provider", "并发 13", "今日 $1.23"]);
    }

    #[test]
    fn compact_units_stay_stable() {
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(1_500), "1.50K");
        assert_eq!(format_tokens(507_900_000), "508M");
    }
}
