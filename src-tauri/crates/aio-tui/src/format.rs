use crate::config::StatusItem;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Default,
    Success,
    Warning,
    Error,
    Scope,
    Provider,
    Model,
    Folder,
    Timing,
    Cost,
    Activity,
    Tokens,
    Version,
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSegment {
    pub text: String,
    pub tone: StatusTone,
}

impl StatusSegment {
    pub fn new(text: impl Into<String>, tone: StatusTone) -> Self {
        Self {
            text: text.into(),
            tone,
        }
    }
}

pub fn status_segments(snapshot: &ObserverSnapshotV1, items: &[StatusItem]) -> Vec<StatusSegment> {
    items
        .iter()
        .map(|item| status_segment(snapshot, *item))
        .collect()
}

fn status_segment(snapshot: &ObserverSnapshotV1, item: StatusItem) -> StatusSegment {
    match item {
        StatusItem::Gateway => {
            if snapshot.gateway.running {
                let text = snapshot
                    .gateway
                    .port
                    .map(|port| format!("网关 {port}"))
                    .unwrap_or_else(|| "网关 开启".to_string());
                StatusSegment::new(text, StatusTone::Success)
            } else {
                StatusSegment::new("网关 关闭", StatusTone::Error)
            }
        }
        StatusItem::Scope => StatusSegment::new(
            format!("范围 {}", scope_label(snapshot.scope)),
            StatusTone::Scope,
        ),
        StatusItem::PreferredProvider => {
            if !snapshot.preferred_provider.available {
                StatusSegment::new("首选 不可用", StatusTone::Warning)
            } else if let Some(provider) = snapshot.preferred_provider.value.as_ref() {
                let text = if snapshot.scope == CliScope::All {
                    format!(
                        "首选 {}/{}",
                        cli_label(&provider.cli_key),
                        provider.provider_name
                    )
                } else {
                    format!("首选 {}", provider.provider_name)
                };
                StatusSegment::new(text, StatusTone::Provider)
            } else {
                StatusSegment::new("首选 —", StatusTone::Default)
            }
        }
        StatusItem::LastRequest => last_request_field(snapshot, "上次", |request| {
            (
                format!(
                    "{} {} {}",
                    terminal_status(request),
                    request.provider_name.as_deref().unwrap_or("—"),
                    route_result(request)
                ),
                request_tone(request),
            )
        }),
        StatusItem::LastStatus => last_request_field(snapshot, "状态", |request| {
            (terminal_status(request), request_tone(request))
        }),
        StatusItem::LastProvider => last_request_field(snapshot, "上游", |request| {
            (
                request.provider_name.as_deref().unwrap_or("—").to_string(),
                StatusTone::Provider,
            )
        }),
        StatusItem::LastRoute => last_request_field(snapshot, "路由", |request| {
            (route_result(request), route_tone(request))
        }),
        StatusItem::LastModel => last_request_field(snapshot, "模型", |request| {
            (
                request.model.as_deref().unwrap_or("—").to_string(),
                StatusTone::Model,
            )
        }),
        StatusItem::LastFolder => last_request_field(snapshot, "目录", |request| {
            (
                request.folder_name.as_deref().unwrap_or("—").to_string(),
                StatusTone::Folder,
            )
        }),
        StatusItem::LastDuration => last_request_field(snapshot, "耗时", |request| {
            (
                request
                    .duration_ms
                    .map(format_duration)
                    .unwrap_or_else(|| "—".to_string()),
                StatusTone::Timing,
            )
        }),
        StatusItem::LastTtfb => last_request_field(snapshot, "首字", |request| {
            (
                request
                    .ttfb_ms
                    .map(format_duration)
                    .unwrap_or_else(|| "—".to_string()),
                StatusTone::Timing,
            )
        }),
        StatusItem::LastCost => last_request_field(snapshot, "费用", |request| {
            (
                request
                    .cost_usd
                    .map(format_cost)
                    .unwrap_or_else(|| "—".to_string()),
                StatusTone::Cost,
            )
        }),
        StatusItem::RecentProvider => {
            if !snapshot.dominant_provider.available {
                StatusSegment::new("近10 不可用", StatusTone::Warning)
            } else if let Some(provider) = snapshot.dominant_provider.value.as_ref() {
                StatusSegment::new(
                    format!("近10 {}×{}", provider.provider_name, provider.count),
                    StatusTone::Provider,
                )
            } else {
                StatusSegment::new("近10 —", StatusTone::Default)
            }
        }
        StatusItem::Concurrency => StatusSegment::new(
            format!("并发 {}", snapshot.active_inference_count),
            StatusTone::Activity,
        ),
        StatusItem::TodayCost => {
            if !snapshot.today.available {
                StatusSegment::new("今日 不可用", StatusTone::Warning)
            } else {
                StatusSegment::new(
                    format!(
                        "今日 {}",
                        snapshot
                            .today
                            .value
                            .as_ref()
                            .and_then(|value| value.cost_usd)
                            .map(format_cost)
                            .unwrap_or_else(|| "—".to_string())
                    ),
                    StatusTone::Cost,
                )
            }
        }
        StatusItem::TodayTokens => {
            if !snapshot.today.available {
                StatusSegment::new("Token 不可用", StatusTone::Warning)
            } else {
                StatusSegment::new(
                    format!(
                        "Token {}",
                        snapshot
                            .today
                            .value
                            .as_ref()
                            .map(|value| format_tokens(value.total_tokens))
                            .unwrap_or_else(|| "—".to_string())
                    ),
                    StatusTone::Tokens,
                )
            }
        }
        StatusItem::AppVersion => {
            StatusSegment::new(format!("AIO {}", snapshot.app_version), StatusTone::Version)
        }
    }
}

fn last_request_field(
    snapshot: &ObserverSnapshotV1,
    label: &str,
    render: impl FnOnce(&ObserverRequest) -> (String, StatusTone),
) -> StatusSegment {
    if !snapshot.last_request.available {
        return StatusSegment::new(format!("{label} 不可用"), StatusTone::Warning);
    }
    let Some(request) = snapshot.last_request.value.as_ref() else {
        return StatusSegment::new(format!("{label} —"), StatusTone::Default);
    };
    let (value, tone) = render(request);
    StatusSegment::new(format!("{label} {value}"), tone)
}

fn request_tone(request: &ObserverRequest) -> StatusTone {
    if request.state == ObserverRequestState::Active {
        StatusTone::Activity
    } else if request.interrupted {
        StatusTone::Warning
    } else if request
        .status
        .is_some_and(|status| (200..300).contains(&status))
    {
        StatusTone::Success
    } else {
        StatusTone::Error
    }
}

fn route_tone(request: &ObserverRequest) -> StatusTone {
    if request.provider_switch_count > 0 || request.retry_count > 0 {
        StatusTone::Warning
    } else if request.attempt_count > 0 {
        StatusTone::Success
    } else {
        StatusTone::Default
    }
}

pub fn status_plain(snapshot: &ObserverSnapshotV1, items: &[StatusItem]) -> String {
    status_segments(snapshot, items)
        .into_iter()
        .map(|segment| segment.text)
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn wrap_status_segments(segments: &[StatusSegment], width: usize) -> Vec<Vec<StatusSegment>> {
    if width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = Vec::new();
    let mut current_width = 0_usize;
    for segment in segments {
        let pieces = wrap_display(&segment.text, width);
        for (index, piece) in pieces.into_iter().enumerate() {
            if index > 0 && !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            let separator_width = if current.is_empty() { 0 } else { 3 };
            let candidate_width = current_width
                .saturating_add(separator_width)
                .saturating_add(display_width(&piece));
            if candidate_width <= width {
                if !current.is_empty() {
                    current.push(StatusSegment::new(" · ", StatusTone::Separator));
                }
                current.push(StatusSegment::new(piece, segment.tone));
                current_width = candidate_width;
            } else {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                current_width = display_width(&piece);
                current = vec![StatusSegment::new(piece, segment.tone)];
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub fn truncate_status_line(line: &[StatusSegment], max_width: usize) -> Vec<StatusSegment> {
    let width = line
        .iter()
        .map(|segment| display_width(&segment.text))
        .sum::<usize>();
    if width <= max_width {
        return line.to_vec();
    }
    if max_width == 0 {
        return Vec::new();
    }
    let target = max_width - 1;
    let mut output = Vec::new();
    let mut used = 0_usize;
    'segments: for segment in line {
        for grapheme in segment.text.graphemes(true) {
            let next = grapheme.width();
            if used.saturating_add(next) > target {
                break 'segments;
            }
            push_status_text(&mut output, grapheme, segment.tone);
            used = used.saturating_add(next);
        }
    }
    push_status_text(&mut output, "…", StatusTone::Separator);
    output
}

fn push_status_text(output: &mut Vec<StatusSegment>, text: &str, tone: StatusTone) {
    if let Some(last) = output.last_mut().filter(|last| last.tone == tone) {
        last.text.push_str(text);
    } else {
        output.push(StatusSegment::new(text, tone));
    }
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
        if request.retry_count > 0 {
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
    let mut width: usize = 0;
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
    let mut width: usize = 0;
    for grapheme in value.graphemes(true) {
        let next = grapheme.width();
        if next > max_width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            lines.push(truncate_display(grapheme, max_width));
            width = 0;
            continue;
        }
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
    use aio_observer_protocol::{
        ObserverDominantProvider, ObserverGatewayStatus, ObserverPreferredProvider,
        ObserverSection, ObserverTodayUsage, OBSERVER_PROTOCOL_VERSION,
    };

    fn request_with_route_counts(
        attempt_count: u32,
        retry_count: u32,
        provider_switch_count: u32,
    ) -> ObserverRequest {
        ObserverRequest {
            key: "request-1".to_string(),
            state: ObserverRequestState::Terminal,
            cli_key: "codex".to_string(),
            method: "POST".to_string(),
            path: "/v1/responses".to_string(),
            model: Some("gpt-5".to_string()),
            provider_name: Some("Provider".to_string()),
            status: Some(200),
            error_code: None,
            interrupted: false,
            created_at_ms: 1,
            last_activity_ms: 2,
            duration_ms: Some(1),
            ttfb_ms: Some(1),
            attempt_count,
            retry_count,
            provider_switch_count,
            has_failover: provider_switch_count > 0,
            session_reuse: false,
            session_id: None,
            folder_name: None,
            usage: None,
            cost_usd: None,
            route: Vec::new(),
            context_compaction: None,
        }
    }

    fn status_snapshot() -> ObserverSnapshotV1 {
        let mut request = request_with_route_counts(3, 1, 1);
        request.provider_name = Some("happy".to_string());
        request.model = Some("gpt-5.6-sol".to_string());
        request.folder_name = Some("aio-coding-hub".to_string());
        request.duration_ms = Some(12_345);
        request.ttfb_ms = Some(2_345);
        request.cost_usd = Some(0.123456);
        ObserverSnapshotV1 {
            protocol_version: OBSERVER_PROTOCOL_VERSION,
            app_version: "0.60.39".to_string(),
            generated_at_ms: 1,
            scope: CliScope::Codex,
            gateway: ObserverGatewayStatus {
                running: true,
                port: Some(37123),
            },
            preferred_provider: ObserverSection::ready(ObserverPreferredProvider {
                cli_key: "codex".to_string(),
                provider_name: "preferred".to_string(),
                circuit_state: "closed".to_string(),
            }),
            last_request: ObserverSection::ready(request),
            dominant_provider: ObserverSection::ready(ObserverDominantProvider {
                provider_name: "happy".to_string(),
                count: 7,
                sample_size: 10,
            }),
            active_inference_count: 13,
            today: ObserverSection::ready(ObserverTodayUsage {
                total_tokens: 507_900_000,
                cost_usd: Some(12.34),
            }),
            active_requests: ObserverSection::ready(Vec::new()),
            recent_requests: ObserverSection::ready(Vec::new()),
            providers: None,
        }
    }

    #[test]
    fn truncation_respects_cjk_display_width() {
        assert_eq!(truncate_display("供应商OpenAI", 8), "供应商O…");
        assert!(display_width(&truncate_display("供应商OpenAI", 8)) <= 8);
    }

    #[test]
    fn selected_status_items_keep_requested_order_and_plain_output() {
        let snapshot = status_snapshot();
        let items = [
            StatusItem::LastModel,
            StatusItem::Gateway,
            StatusItem::Concurrency,
            StatusItem::TodayCost,
        ];
        let segments = status_segments(&snapshot, &items);
        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>(),
            vec!["模型 gpt-5.6-sol", "网关 37123", "并发 13", "今日 $12.340"]
        );
        assert_eq!(
            status_plain(&snapshot, &items),
            "模型 gpt-5.6-sol | 网关 37123 | 并发 13 | 今日 $12.340"
        );
        assert_eq!(segments[0].tone, StatusTone::Model);
        assert_eq!(segments[1].tone, StatusTone::Success);
    }

    #[test]
    fn every_status_catalog_item_has_a_projection() {
        let segments = status_segments(&status_snapshot(), &StatusItem::ALL);
        assert_eq!(segments.len(), StatusItem::ALL.len());
        assert!(segments.iter().all(|segment| !segment.text.is_empty()));
    }

    #[test]
    fn status_segments_wrap_at_boundaries_first() {
        let lines = wrap_status_segments(
            &[
                StatusSegment::new("首选 Provider", StatusTone::Provider),
                StatusSegment::new("并发 13", StatusTone::Activity),
                StatusSegment::new("今日 $1.23", StatusTone::Cost),
            ],
            16,
        );
        let text = lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|segment| segment.text.as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        assert_eq!(text, vec!["首选 Provider", "并发 13", "今日 $1.23"]);
        assert_eq!(lines[0][0].tone, StatusTone::Provider);
        assert_eq!(lines[1][0].tone, StatusTone::Activity);
        assert_eq!(lines[2][0].tone, StatusTone::Cost);
    }

    #[test]
    fn styled_status_wrap_preserves_cjk_width_and_separator_tone() {
        let lines = wrap_status_segments(
            &[
                StatusSegment::new("模型 编程助手", StatusTone::Model),
                StatusSegment::new("今日 $1.23", StatusTone::Cost),
                StatusSegment::new("并发 13", StatusTone::Activity),
            ],
            26,
        );
        assert!(lines.iter().all(|line| {
            line.iter()
                .map(|segment| display_width(&segment.text))
                .sum::<usize>()
                <= 26
        }));
        assert!(lines
            .iter()
            .flatten()
            .any(|segment| { segment.text == " · " && segment.tone == StatusTone::Separator }));
        assert!(lines
            .iter()
            .flatten()
            .any(|segment| segment.tone == StatusTone::Model));
        assert!(lines
            .iter()
            .flatten()
            .any(|segment| segment.tone == StatusTone::Cost));
    }

    #[test]
    fn styled_truncation_keeps_tones_and_fits() {
        let line = vec![
            StatusSegment::new("模型 中文模型", StatusTone::Model),
            StatusSegment::new(" · ", StatusTone::Separator),
            StatusSegment::new("今日 $12.34", StatusTone::Cost),
        ];
        let truncated = truncate_status_line(&line, 12);
        let width = truncated
            .iter()
            .map(|segment| display_width(&segment.text))
            .sum::<usize>();
        assert!(width <= 12);
        assert_eq!(
            truncated.last().map(|segment| segment.text.as_str()),
            Some("…")
        );
        assert_eq!(truncated[0].tone, StatusTone::Model);
    }

    #[test]
    fn compact_units_stay_stable() {
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(1_500), "1.50K");
        assert_eq!(format_tokens(507_900_000), "508M");
    }

    #[test]
    fn route_result_keeps_switches_and_retries_independent() {
        assert_eq!(
            route_result(&request_with_route_counts(3, 1, 1)),
            "切换1/重试1"
        );
        assert_eq!(route_result(&request_with_route_counts(2, 0, 1)), "切换1");
        assert_eq!(route_result(&request_with_route_counts(3, 2, 0)), "重试2");
        assert_eq!(route_result(&request_with_route_counts(0, 0, 0)), "未上游");
    }
}
